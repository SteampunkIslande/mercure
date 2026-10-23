use anyhow::{Context, Result, bail};
use log::{error, info, warn};
use nix::sys::signal::Signal;
use smaug::config::{SmaugConfig, get_smaug_config};
use smaug::models::{ModelError, SigintBehavior, analysis, appstate};
use smaug_lib::pipeline_exec::versionning;
use std::io::Error as IoError;
use std::path::{self, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::str::FromStr;
use tokio::task::{JoinError as TokioJoinError, JoinHandle, JoinSet};

use sqlx::SqlitePool;

use wherror::Error;

use regex::Error as RegexError;
use smaug_lib::models::AnalysisStateMachineError;
use sqlx::Error;

use smaug_lib::models::Attempt;

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
    #[error("{location}: {source}")]
    GitCheckError {
        #[from]
        source: versionning::GitCheckError,
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
                        match analysis::start_run_analysis(attempt.run_id, &pool_clone).await {
                            // OK: Tout va bien
                            Ok(()) => {
                                info!("Lancement de la tentative {} du run {}", attempt.attempt_number, attempt.run_id);
                            },
                            // Erreur au moment de start run: tenter de faire remonter cette erreur
                            Err(e) => {
                                // Impossible d'écrire l'erreur dans la base: très fâcheux, écrire dans stder en dernier recours
                                if let Err(e) = analysis::fail_cannot_analyse_run(attempt.run_id, &e.to_string(), &pool_clone).await {
                                    error!("{}", e);
                                }
                            }
                        }
                        match run_script(&attempt, &pool_clone, child_rx).await {
                            Ok(()) => {
                                info!("Tentative {} du run {} terminée", attempt.attempt_number, attempt.attempt_number);
                                if let Err(e) = analysis::complete_success(attempt.run_id, &pool_clone).await {
                                    error!("{}", e);
                                }
                            }
                            // D'abord on essaye d'écrire l'erreur dans la base de données, mais si la transition est invalide, on l'affiche dans stderr
                            Err(e) => {
                                if let Err(e) = analysis::complete_failure(attempt.run_id, &e.to_string(), &pool_clone).await {
                                    error!("{}", e)
                                }
                            }
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

/// Cette fonction peut paniquer, ou retourner une erreur, les conséquences seront minimes
///
/// Si elle panique, c'est juste une tâche tokio parmi d'autres, qui faisait partie du JoinSet global de la routine (le set qui stocke tous les jobs en cours)
async fn run_script(
    attempt: &Attempt,
    pool: &SqlitePool,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), RoutineError> {
    let (git_manager, scripts_dir, logs_dir) = {
        let SmaugConfig {
            scripts_dir,
            logs_dir,
            pipelines,
            ..
        } = get_smaug_config();
        let scripts_dir = PathBuf::from_str(&scripts_dir).context("Le paramètre `scripts_dir` de la configuration actuelle ne peut pas être convertie en un chemin")?;
        if !scripts_dir.is_absolute() {
            return Err(anyhow::format_err!(
                "Le dossier `scripts_dir` est mal configuré, un chemin absolu est requis"
            )
            .into());
        }
        if !scripts_dir.exists() {
            std::fs::create_dir_all(&scripts_dir)
                .with_context(|| format!("Impossible de créer le dossier des scripts"))?;
        }

        let logs_dir = PathBuf::from_str(&logs_dir).context("Le paramètre `logs_dir` de la configuration actuelle ne peut pas être convertie en un chemin")?;
        if !logs_dir.is_absolute() {
            return Err(anyhow::format_err!(
                "Le dossier `logs_dir` est mal configuré, un chemin absolu est requis"
            )
            .into());
        }
        if !logs_dir.exists() {
            std::fs::create_dir_all(&logs_dir)
                .with_context(|| format!("Impossible de créer le dossier des logs"))?;
        }
        (pipelines, scripts_dir, logs_dir)
    };

    let script_id = attempt.attempt_full_name();

    let script_path = scripts_dir.join(path::Path::new(&format!("{}.sh", script_id)));
    let stdout_path = logs_dir.join(path::Path::new(&format!("{}.stdout", script_id)));
    let stderr_path = logs_dir.join(path::Path::new(&format!("{}.stderr", script_id)));

    let rendered = attempt.render_script().await?;
    std::fs::create_dir_all(attempt.outdir())
        .with_context(|| format!("Impossible de créer le dossier outdir"))?;
    std::fs::write(&script_path, &rendered)?;

    // clones the whole repo into the working directory
    versionning::git_clone_to(&attempt.outdir(), &attempt.form_identification, git_manager).await?;

    let mut child = match Command::new("bash")
        .arg("-l") // Important, pour que l'environnement d'exécution soit celui d'un shell login (reproductibilité si relancé avec le même utilisateur)
        .arg(&script_path)
        .current_dir(attempt.outdir()) // Pour l'exécution du script, on se place dans outdir
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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
    let stdout = child.stdout.take().ok_or(anyhow::format_err!("Fuck"))?;
    let stderr = child.stderr.take().ok_or(anyhow::format_err!("Fuck"))?;

    let stdout_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        let mut file = tokio::fs::File::create(&stdout_path).await?;
        let mut stdout = stdout;
        tokio::io::copy(&mut stdout, &mut file).await?;
        Ok(())
    });

    let stderr_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        let mut file = tokio::fs::File::create(&stderr_path).await?;
        let mut stderr = stderr;
        tokio::io::copy(&mut stderr, &mut file).await?;
        Ok(())
    });

    // Compétition entre la fin du script et le signal d'interruption
    tokio::select! {
        status_res = child.wait() => {
            stdout_handle.await??;
            stderr_handle.await??;
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
                    eprintln!("Waiting for job completion...");
                    match child.wait().await {
                        Ok(status) if status.success() => eprintln!("Le job s'est terminé avec succès"),
                        Ok(status) => eprintln!("Le job s'est terminé avec le status: {}", status.code().unwrap_or(-1)),
                        Err(e) => eprintln!("Erreur en attendant la fin du job: {}", e),
                    }
                }
            }
        }
    }
    Ok(())
}
