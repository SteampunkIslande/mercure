use anyhow::Result;
use log::{error, info, warn};
use mercure::models::{HgRun, ModelError};
use mercure_lib::config::get_mercure_config;
use std::io::Error as IoError;
use tokio::task::{JoinError as TokioJoinError, JoinSet};

use sqlx::SqlitePool;

use std::path::{Path, PathBuf};
use thiserror::Error;

use chrono::ParseError;
use mercure_lib::models::AnalysisStateMachineError;
use regex::{Error as RegexError, Regex};
use sqlx::Error;

use mercure_lib::models::HgAttempt;

use tokio::process::Command;
use tokio::sync::broadcast;
use tokio::time::{Duration, interval};

#[derive(Error, Debug)]
enum RoutineError {
    #[error(transparent)]
    Sqlx(#[from] Error),
    #[error(transparent)]
    AnalysisStateMachine(#[from] AnalysisStateMachineError),
    #[error(transparent)]
    DateParse(#[from] ParseError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    IO(#[from] IoError),
    #[error(transparent)]
    TokioJoin(#[from] TokioJoinError),
    #[error(transparent)]
    InvalidRegex(#[from] RegexError),
    #[error("Le dossier d'entrée {0} est introuvable")]
    IndirNotFound(String),
    #[error("Aucun dossier d'entrée spécifié")]
    NoIndir,
    #[error("Impossible de déterminer si le run est terminé: {0}")]
    CheckRunCompletedError(String),
}

pub async fn run_routine_loop(
    pool: SqlitePool,
    mut global_shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    // Ce canal sert toujours à prévenir les *processus enfants* de la routine
    let (child_shutdown_tx, _) = broadcast::channel(1);
    let mut join_set = JoinSet::new();

    let mut interval = interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            // --- Interception du signal centralisé envoyé depuis web.rs ---
            _ = global_shutdown_rx.recv() => {
                println!("\n🛑 Signal d'arrêt reçu par la routine...");
                break;
            }

            // --- Logique de polling toutes les 2 secondes ---
            _ = interval.tick() => {
                if let Some(script) = find_run_to_launch(&pool).await {
                    let child_rx = child_shutdown_tx.subscribe();

                    // Lancement de la tâche dans le JoinSet
                    join_set.spawn(async move {
                        run_analysis(&script, child_rx).await;
                    });
                }
            }
        }
    }

    // Réveille tous les `child_rx.recv()` dans run_analysis
    let _ = child_shutdown_tx.send(());

    if join_set.is_empty() {
        info!("Aucun job n'était en train de tourner");
    } else {
        info!("⏳ Attente de l'arrêt propre des processus enfants...");
    }

    // On attend que chaque tâche ait terminé son bloc de code (incluant le kill et le rename)
    while let Some(res) = join_set.join_next().await {
        if let Err(e) = res {
            warn!("Un des jobs s'est terminé avec une erreur : {}", e);
        }
    }

    println!("👋 Tous les processus ont été arrêtés. Fin de la routine.");

    Ok(())
}

async fn find_run_to_launch(_pool: &SqlitePool) -> Option<String> {
    None
}

async fn run_analysis(_script_content: &str, mut shutdown_rx: broadcast::Receiver<()>) {
    let mut child = match Command::new("bash").spawn() {
        Ok(c) => c,
        Err(e) => {
            error!("Erreur de lancement pour {:?} : {}", "nom du run...", e);
            return;
        }
    };

    // Compétition entre la fin du script et le signal d'interruption
    tokio::select! {
        status_res = child.wait() => {
            // Le processus s'est terminé de lui-même
            match status_res {
                Ok(status) if status.success() => {
                    println!("Le processus s'est bien lancé. Penser à écrire dans la base de données...");
                }
                Ok(status) => {
                    println!("Le processus s'est terminé avec une erreur... Code d'erreur: {} Il faudrait écrire ça dans la base de données aussi...",status.code().and_then(|c|Some(c.to_string())).unwrap_or("?".to_string()));
                }
                Err(e) => {
                    eprintln!("Erreur lors de l'attente de la fin de l'analyse... {}",  e);
                }
            }
        }

        _ = shutdown_rx.recv() => {

            // Terminer le processus
            if let Err(e) = child.kill().await {
                error!("Erreur au moment de terminer le processus : {}",  e);
            }
        }
    }
}

fn get_indir_outdir_for_analysisdir(
    attempt: &HgAttempt,
) -> Result<(PathBuf, PathBuf), RoutineError> {
    let indir_str = attempt.indir.as_ref().ok_or(RoutineError::NoIndir)?;
    Ok((PathBuf::from(&indir_str), PathBuf::from(&indir_str)))
}

fn get_indir_outdir_for_ontdir(
    attempt: &HgAttempt,
    run: &HgRun,
) -> Result<(PathBuf, PathBuf), RoutineError> {
    let indir_str = attempt.indir.as_ref().ok_or(RoutineError::NoIndir)?;

    let config = get_mercure_config();

    let run_date_short = attempt.run_date[2..].replace("-", "");
    let seq_name = &attempt.run_sequencer;
    let run_name = &run.run_name;

    Ok((
        PathBuf::from(&indir_str),
        PathBuf::from(config.analysis_dir).join(format!("{run_date_short}_{seq_name}_{run_name}")),
    ))
}

fn get_indir_outdir_for_illumina(
    attempt: &HgAttempt,
    _run: &HgRun,
) -> Result<(PathBuf, PathBuf), RoutineError> {
    // Le dossier de run brut Illumina devrait être dans {sequencers_dir}/{run_sequencer}/output/{run_date}_{run_sequencer}_*_{run_flowcellid}*
    let config = get_mercure_config();
    let raw_dir = Path::new(&config.sequencers_dir)
        .join(&attempt.run_sequencer)
        .join("output");

    let run_date_short = attempt.run_date[2..].replace("-", "");
    let seq_name = &attempt.run_sequencer;
    let flowcell_id = &attempt.run_flowcellid;

    let run_dir_pattern = format!(r"^{run_date_short}_{seq_name}_(\d+)_.+{flowcell_id}$");
    let run_dir_re = Regex::new(&run_dir_pattern)?;

    let (bcl_dir_base, seq_run_counter) = std::fs::read_dir(&raw_dir)?
        .filter_map(|e| {
            e.map(|e| {
                run_dir_re
                    .captures(&e.file_name().display().to_string())
                    .and_then(|cap| {
                        cap.get(1).and_then(|c| {
                            Some((
                                e.file_name().display().to_string(),
                                c.as_str().to_string().parse::<i64>().ok()?,
                            ))
                        })
                    })
            })
            .ok()?
        })
        .next()
        .ok_or_else(|| RoutineError::IndirNotFound(run_dir_pattern))?;

    let output_dir = PathBuf::from(&config.analysis_dir).join(format!(
        "{}_{}_{}",
        run_date_short, seq_name, seq_run_counter
    ));

    Ok((raw_dir.join(bcl_dir_base), output_dir))
}
