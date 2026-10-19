use anyhow::{Context, Result, bail};
use log::{error, info, warn};
use mercure::config::{MercureConfig, get_mercure_config};
use mercure::models::{ModelError, SigintBehavior, analysis, appstate};
use nix::sys::signal::Signal;
use std::io::Error as IoError;
use std::path::{self, PathBuf};
use std::process::ExitStatus;
use std::str::FromStr;
use tokio::task::{JoinError as TokioJoinError, JoinSet};

use sqlx::SqlitePool;

use wherror::Error;

use mercure_lib::models::AnalysisStateMachineError;
use regex::Error as RegexError;
use sqlx::Error;

use mercure_lib::models::Attempt;

use tokio::process::{Child, Command};
use tokio::sync::broadcast;
use tokio::time::{Duration, interval};

use nix::{sys::signal::kill, unistd::Pid};

#[derive(Error, Debug)]
enum RoutineError {
    #[error("{location}: {source}")]
    Sqlx {
        #[from]
        source: Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    AnalysisStateMachine {
        #[from]
        source: AnalysisStateMachineError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    Model {
        #[from]
        source: ModelError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    IO {
        #[from]
        source: IoError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    TokioJoin {
        #[from]
        source: TokioJoinError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    InvalidRegex {
        #[from]
        source: RegexError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    AnyhowError {
        #[from]
        source: anyhow::Error,
        location: &'static std::panic::Location<'static>,
    },
}

pub async fn run_routine_loop(
    pool: &SqlitePool,
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
                let mut runs_to_launch = find_run_to_launch(pool).await?;

                while let Some(attempt) = runs_to_launch.pop() {
                    let child_rx = child_shutdown_tx.subscribe();

                    // 1. Clone the pool handle. This is cheap (Arc clone under the hood).
                    let pool_clone = pool.clone();

                    // 2. Move the cloned handle into the spawned task
                    join_set.spawn(async move {
                        match run_script(&attempt, &pool_clone, child_rx).await {
                            Ok(()) => { info!("Run terminé"); }
                            Err(e) => { error!("{}", e.to_string()); }
                        }
                    });
                }
            }
        }
    }

    match child_shutdown_tx.send(()) {
        Ok(n) => {
            info!(
                "{} jobs sont encore en cours, leur task tokio est informée.",
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

async fn find_run_to_launch(pool: &SqlitePool) -> Result<Vec<Attempt>, RoutineError> {
    Ok(Attempt::get_pending_runs(pool).await?)
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

async fn run_script(
    attempt: &Attempt,
    pool: &SqlitePool,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), RoutineError> {
    let scripts_dir = {
        let MercureConfig { scripts_dir, .. } = get_mercure_config();
        let scripts_dir = PathBuf::from_str(&scripts_dir).context("Le paramètre `scripts_dir` de la configuration actuelle ne peut pas être convertie en un chemin")?;
        if !scripts_dir.exists() {
            std::fs::create_dir_all(&scripts_dir)
                .with_context(|| format!("Impossible de créer le dossier des scripts"))?;
        }
        scripts_dir
    };

    let script_id = format!(
        "{}-run-{:0<8}-tentative-{:0<8}",
        time::Timestamp::now().as_seconds(),
        attempt.run_id,
        attempt.attempt_number
    );

    let script_path = scripts_dir.join(path::Path::new(&format!("{}.sh", script_id)));

    let rendered = attempt.render_script().await?;
    std::fs::create_dir_all(attempt.outdir())
        .with_context(|| format!("Impossible de créer le dossier outdir"))?;
    std::fs::write(&script_path, &rendered)?;

    let mut child = match Command::new("bash")
        .arg(&script_path)
        .process_group(0)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return Err(anyhow::format_err!(
                "Erreur de lancement pour la tentative {} du run {}. Erreur: `{}`",
                attempt.attempt_number,
                attempt.run_id,
                e
            )
            .into());
        }
    };

    // Compétition entre la fin du script et le signal d'interruption
    tokio::select! {
        status_res = child.wait() => {
            // Le processus s'est terminé de lui-même
            match status_res {
                Ok(status) if status.success() => {
                    analysis::complete_success(attempt.run_id, pool).await?;
                }
                Ok(status) => {
                    analysis::complete_failure(
                        attempt.run_id,
                        &format!(
                            "(exit code {})",
                            &status
                                .code()
                                .map(|c| c.to_string())
                                .unwrap_or("?".to_string())
                        ),
                        pool,
                    )
                    .await?;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
            Ok(())
        }

        _ = shutdown_rx.recv() => {
            eprintln!("Attention, la tentative {} du run {} a reçu un signal d'arrêt prématuré", attempt.attempt_number, attempt.run_id);
            let sigint_behavior: appstate::SigintBehavior = appstate::AppState::get(pool).await?;
            match sigint_behavior {
                SigintBehavior::StopJobs => match send_sigint_then_kill(&mut child, None).await {
                    Ok(status) if status.success() => {
                        eprintln!("OK. On a envoyé SIGINT mais tout va bien, status 0.");
                    }
                    Ok(status) => {
                        eprintln!("Ooops. Status: {}", status);
                    }
                    Err(e) => {
                        eprintln!("Double oops: {}", e);
                    }
                },
                SigintBehavior::WaitForJobCompletion => {
                    match child.wait().await {
                        Ok(status) => eprintln!("Le job s'est finalement terminé avec le status: {}", status),
                        Err(e) => eprintln!("Erreur en attendant la fin du job: {}", e),
                    }
                }
            }
            Ok(())
        }
    }
}
