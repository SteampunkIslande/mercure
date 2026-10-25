use anyhow::{Context, Result, bail};
use log::{error, info, warn};
use smaug::config::get_smaug_config;
use smaug::models::{ModelError, SigintBehavior, analysis, appstate};
use smaug_lib::pipeline_exec::{LogProgressParser, RunProgressCache, versionning};
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::task::{JoinError as TokioJoinError, JoinSet};

use sqlx::SqlitePool;

use wherror::Error;

use regex::Error as RegexError;
use smaug_lib::models::AnalysisStateMachineError;
use sqlx::Error;

use smaug_lib::models::Attempt;

use tokio::process::{Child, Command};
use tokio::sync::broadcast;
use tokio::time::{Duration, interval};

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
    progress_cache: RunProgressCache,
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
                if !matches!(appstate::AppState::get(pool).await.unwrap_or_default(), appstate::ActiveRoutine::RoutineOn) {
                    continue;
                }
                match find_run_to_launch(pool).await{

                    Ok(mut runs_to_launch) => {
                        while let Some(attempt) = runs_to_launch.pop() {
                            let child_rx = child_shutdown_tx.subscribe();

                            // 1. Clone the pool handle. This is cheap (Arc clone under the hood).
                            let pool_clone = pool.clone();
                            let progress_cache_clone = progress_cache.clone();

                            // 2. Move the cloned handle into the spawned task
                            join_set.spawn(async move {
                                match analysis::start_run_analysis(attempt.run_id, &pool_clone).await {
                                    // OK: Tout va bien
                                    Ok(_) => {
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
                                match run_script(&attempt, &pool_clone, progress_cache_clone, child_rx).await {
                                    Ok(()) => {
                                        info!("Tentative {} du run {} terminée", attempt.attempt_number, attempt.attempt_number);
                                    }
                                    // C'est run_script qui s'est chargée de mettre à jour la base de données
                                    Err(e) => {
                                        error!("{}", e);
                                    }
                                }
                            });
                        }
                    },
                    Err(e) => {
                        error!("Something serious: cannot run 'find runs to launch' {}", e);
                    }
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
        info!("Tous les jobs ont été arrêtés.");
    }
    info!("Fin de la routine");

    Ok(())
}

async fn find_run_to_launch(pool: &SqlitePool) -> Result<Vec<Attempt>, RoutineError> {
    Ok(Attempt::get_pending_runs(pool).await?)
}

async fn send_sigint_then_kill(
    child: &mut Child,
    grace_time_seconds: Option<u64>,
) -> Result<ExitStatus> {
    use nix::errno::Errno;
    use nix::sys::signal::{Signal, kill, killpg};
    use nix::unistd::{Pid, getpgid};

    if let Some(status) = child.try_wait().context("Failed to poll process status")? {
        return Ok(status);
    }

    let pid = child
        .id()
        .context("Process has no PID (likely already reaped)")?;
    let pid_i32: i32 = pid.try_into().context("Invalid PID magnitude")?;
    let nix_pid = Pid::from_raw(pid_i32);

    // Determine if the child is a process group leader
    let is_group_leader = match getpgid(Some(nix_pid)) {
        Ok(pgid) => pgid == nix_pid,
        Err(Errno::ESRCH) => {
            // Process died before we could check its PGID.
            // We set this to false, the subsequent signal will harmlessly fail with ESRCH.
            false
        }
        Err(e) => bail!("Failed to get process group ID for {}: {}", pid, e),
    };

    // 1. Send SIGINT
    let sigint_result = if is_group_leader {
        killpg(nix_pid, Signal::SIGINT)
    } else {
        kill(nix_pid, Signal::SIGINT)
    };

    if let Err(e) = sigint_result {
        if e != Errno::ESRCH {
            let target = if is_group_leader {
                "process group"
            } else {
                "process"
            };
            bail!("Failed to send SIGINT to {} {}: {}", target, pid, e);
        }
    }

    let grace_time = Duration::from_secs(grace_time_seconds.unwrap_or(5));
    if let Ok(wait_result) = tokio::time::timeout(grace_time, child.wait()).await {
        return wait_result.context("Error waiting for process after SIGINT");
    }

    // 2. Timeout expired: send SIGKILL
    if is_group_leader {
        if let Err(e) = killpg(nix_pid, Signal::SIGKILL) {
            if e != Errno::ESRCH {
                bail!("Failed to send SIGKILL to process group {}: {}", pid, e);
            }
        }
    } else {
        // Standard fallback for single process
        child
            .kill()
            .await
            .context("Failed to send SIGKILL to the process")?;
    }

    // 3. Harvest the process
    child
        .wait()
        .await
        .context("Error harvesting the process after SIGKILL")
}

async fn tee_stream_to_file_and_cache<R>(
    stream: R,
    file_path: &Path,
    progress_cache: RunProgressCache,
    run_id: i64,
    attempt_number: u32,
    stream_type: &'static str,
) -> anyhow::Result<()>
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut file = tokio::fs::File::create(file_path).await?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    let mut parser = if stream_type == "stderr" {
        let parser = LogProgressParser::new(run_id, attempt_number);
        progress_cache.upsert(parser.state().clone()).await;
        Some(parser)
    } else {
        None
    };

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break;
        }

        file.write_all(line.as_bytes()).await?;

        if let Some(parser) = parser.as_mut() {
            parser.parse_line(&line);
            progress_cache.upsert(parser.state().clone()).await;
        }
    }

    file.flush().await?;
    Ok(())
}

/// Cette fonction peut paniquer, ou retourner une erreur, les conséquences seront minimes
///
/// Si elle panique, c'est juste une tâche tokio parmi d'autres, qui faisait partie du JoinSet global de la routine (le set qui stocke tous les jobs en cours)
async fn run_script(
    attempt: &Attempt,
    pool: &SqlitePool,
    progress_cache: RunProgressCache,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<(), RoutineError> {
    let (git_manager, scripts_dir, logs_dir) = {
        let config = get_smaug_config();

        let scripts_dir = PathBuf::from(&config.scripts_dir);
        let logs_dir = PathBuf::from(&config.logs_dir);

        ensure_absolute_dir(&scripts_dir, "scripts_dir").await?;
        ensure_absolute_dir(&logs_dir, "logs_dir").await?;

        (config.pipelines, scripts_dir, logs_dir)
    };

    let script_id = attempt.attempt_full_name();
    let script_path = scripts_dir.join(format!("{}.sh", script_id));
    let stdout_path = logs_dir.join(format!("{}.stdout", script_id));
    let stderr_path = logs_dir.join(format!("{}.stderr", script_id));

    let run = attempt.get_run(pool).await?;
    let rendered = attempt.render_script(&run.run_name).await?;

    // Async FS operations
    tokio::fs::create_dir_all(attempt.outdir())
        .await
        .context("Impossible de créer le dossier outdir")?;
    tokio::fs::write(&script_path, &rendered)
        .await
        .context("Impossible d'écrire le script rendu")?;

    // Clones the whole repo into the working directory. Now, PWD of the rendered script contains a `pipelines` directory.
    versionning::git_clone_to(&attempt.outdir(), &attempt.form_identification, git_manager).await?;

    let mut child = Command::new("bash")
        .arg("-l")
        .arg(&script_path)
        .current_dir(attempt.outdir())
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "Erreur de lancement pour la tentative {} du run {}",
                attempt.attempt_number, attempt.run_id
            )
        })
        .map_err(RoutineError::from)?; // Assuming RoutineError implements From<anyhow::Error>

    let stdout = child.stdout.take().context("Can't take stdout handle")?;
    let stderr = child.stderr.take().context("Can't take stderr handle")?;

    let run_id = attempt.run_id;
    let attempt_number = attempt.attempt_number;
    let progress_cache_stdout = progress_cache.clone();
    let progress_cache_stderr = progress_cache;

    // Spawn multiplexers
    let stdout_handle = tokio::spawn(async move {
        tee_stream_to_file_and_cache(
            stdout,
            &stdout_path,
            progress_cache_stdout,
            run_id,
            attempt_number,
            "stdout",
        )
        .await
    });

    let stderr_handle = tokio::spawn(async move {
        tee_stream_to_file_and_cache(
            stderr,
            &stderr_path,
            progress_cache_stderr,
            run_id,
            attempt_number,
            "stderr",
        )
        .await
    });

    // Compétition entre la fin du script et le signal d'interruption
    tokio::select! {
        status_res = child.wait() => {
            // Await logs to finish flushing
            let _ = tokio::try_join!(stdout_handle, stderr_handle)?;
            // Le processus s'est terminé de lui-même
            match status_res {
                Ok(status) if status.success() => {
                    analysis::complete_success(attempt.run_id, pool).await?;
                }
                Ok(status) => {
                    let code = status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".to_string());
                    analysis::complete_failure(attempt.run_id, &format!("(exit code {})", code), pool).await?;
                }
                Err(e) => return Err(e.into()),
            }
        }

        _ = shutdown_rx.recv() => {
            info!("La tentative {} du run {} a reçu un signal d'arrêt prématuré", attempt.attempt_number, attempt.run_id);
            let sigint_behavior: appstate::SigintBehavior = appstate::AppState::get(pool).await?;

            match sigint_behavior {
                SigintBehavior::StopJobs => {
                    info!("Le job va être arrêté par SIGINT, 5s avant SIGKILL.");
                    match send_sigint_then_kill(&mut child, None).await {
                        Ok(status) if status.success() => {
                            analysis::complete_success(attempt.run_id, pool).await?;
                        }
                        Ok(status) => {
                            analysis::complete_failure(attempt.run_id, &format!("(exit code {})", status.code().map(|s|s.to_string()).unwrap_or("Unknown".into())), pool).await?;
                        }
                        Err(e)=> {
                            eprintln!("Erreur lors de l'envoi du signal: {}", e);
                            analysis::complete_failure(attempt.run_id, &format!("erreur {e}"), pool).await?;
                        }
                    }
                }
                SigintBehavior::WaitForJobCompletion => {
                    eprintln!("Waiting for job completion...");
                    let _ = child.wait().await;
                }
            }

            // Crucial: Wait for logs to finish capturing the shutdown process
            let _ = tokio::try_join!(stdout_handle, stderr_handle)?;
        }
    }

    Ok(())
}

// Helper function to deduplicate directory validation
async fn ensure_absolute_dir(path: &Path, name: &str) -> anyhow::Result<()> {
    if !path.is_absolute() {
        return Err(anyhow::anyhow!(
            "Le dossier `{}` est mal configuré, un chemin absolu est requis",
            name
        ));
    }
    if !path.exists() {
        tokio::fs::create_dir_all(path)
            .await
            .with_context(|| format!("Impossible de créer le dossier {}", name))?;
    }
    Ok(())
}
