use anyhow::{Context, Result, bail};
use log::{error, info, warn};
use mercure::models::{ModelError, Run};
use mercure_lib::config::get_mercure_config;
use nix::sys::signal::Signal;
use std::io::Error as IoError;
use std::process::ExitStatus;
use tokio::task::{JoinError as TokioJoinError, JoinSet};

use sqlx::SqlitePool;

use std::path::{Path, PathBuf};
use thiserror::Error;

use chrono::ParseError;
use mercure_lib::models::AnalysisStateMachineError;
use regex::{Error as RegexError, Regex};
use sqlx::Error;

use mercure_lib::models::Attempt;

use tokio::process::{Child, Command};
use tokio::sync::broadcast;
use tokio::time::{Duration, interval};

use nix::{sys::signal::kill, unistd::Pid};

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
    mut routine_shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    // Canal pour prévenir les processus enfants démarrés par la routine
    let (child_shutdown_tx, _) = broadcast::channel(1);
    let mut join_set = JoinSet::new();

    let mut interval = interval(Duration::from_secs(2));

    loop {
        tokio::select! {

            _ = routine_shutdown_rx.recv() => {
                info!("Signal d'arrêt reçu par la routine...");
                break;
            }

            _ = interval.tick() => {
                if let Some(attempt) = find_run_to_launch(&pool).await {
                    let child_rx = child_shutdown_tx.subscribe();

                    // Lancement de la tâche dans le JoinSet
                    join_set.spawn(async move {
                        run_script(&attempt, child_rx).await;
                    });
                }
            }
        }
    }

    match child_shutdown_tx.send(()) {
        Ok(n) => {
            info!(
                "{} jobs sont encore en cours, un signal SIGINT leur a été envoyé",
                n
            );
        }
        Err(_) => {
            info!("Aucun job n'est en train d'écouter");
        }
    }

    if join_set.is_empty() {
        info!("Aucun job n'était en train de tourner");
    } else {
        info!("Attente de l'arrêt propre des jobs encore en cours");
        while let Some(res) = join_set.join_next().await {
            if let Err(e) = res {
                warn!("Un des jobs s'est terminé avec une erreur : {}", e);
            }
        }
        println!("Tous les jobs ont été arrêtés. Fin de la routine.");
    }

    Ok(())
}

async fn find_run_to_launch(_pool: &SqlitePool) -> Option<Attempt> {
    // Lire dans la base de données, pour trouver une tentative dans l'état 'Pending'
    None
}

async fn send_sigint_then_kill(
    child: &mut Child,
    grace_time_seconds: Option<u64>,
) -> Result<ExitStatus> {
    let grace_time = Duration::from_secs(grace_time_seconds.unwrap_or(5));

    let pid = match child.id() {
        Some(id) => id,
        None => bail!("Le processus n'a pas de PID (il a probablement déjà été récolté)"),
    };

    let nix_pid = Pid::from_raw(pid.try_into().context("PID invalide")?);
    if let Err(e) = kill(nix_pid, Signal::SIGINT) {
        bail!("Impossible d'envoyer SIGINT au processus {}: {}", pid, e);
    }

    // Première chance (SIGINT)...
    match tokio::time::timeout(grace_time, child.wait()).await {
        Ok(wait_result) => {
            // Le timeout n'a pas expiré : tout va bien, on retourne le statut après un SIGINT
            Ok(wait_result.context("Erreur lors de l'attente du processus")?)
        }
        Err(_) => {
            // SIGINT n'a pas suffi après le grace_time: KILL (envoi de SIGKILL)
            child
                .kill()
                .await
                .context("Impossible d'envoyer SIGKILL au processus")?;

            // On récupère le statut
            Ok(child
                .wait()
                .await
                .context("Erreur lors de la récolte du processus après SIGKILL")?)
        }
    }
}

async fn run_script(attempt: &Attempt, mut shutdown_rx: broadcast::Receiver<()>) {
    let mut child = match Command::new("bash").spawn() {
        Ok(c) => c,
        Err(e) => {
            error!(
                "Erreur de lancement pour la tentative {} du run {}. Erreur: `{}`",
                attempt.attempt_number, attempt.run_id, e
            );
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
            eprintln!("Attention, la tentative {} du run {} a reçu un signal d'arrêt prématuré", attempt.attempt_number, attempt.run_id);
            match send_sigint_then_kill(&mut child, None).await
            {
                Ok(status) if status.success() =>{
                    eprintln!("OK. On a envoyé SIGINT mais tout va bien, status 0.");
                },
                Ok(status)=>{
                    eprintln!("Ooops. Status: {}",status);
                },
                Err(e)=>{
                    eprintln!("Double oops: {}",e);
                }
            }
        }
    }
}
