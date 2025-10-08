use std::str::FromStr;

use futures::StreamExt;
use mercure::models::AnalysisStateMachineError;
use sqlx;
use sqlx::Row;
use thiserror::Error;
use tokio::sync::watch;

use mercure::models::HgAttempt;
use mercure::models::InvalidRunStatusError;
use mercure::models::RunStatus;

use mercure::models::analysis;

#[derive(Error, Debug)]
enum RoutineError {
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error(transparent)]
    InvalidRunStatusError(#[from] InvalidRunStatusError),
    #[error(transparent)]
    AnalysisStateMachineError(#[from] AnalysisStateMachineError),
}

async fn find_pending_runs(pool: &sqlx::SqlitePool) -> Vec<Result<HgAttempt, RoutineError>> {
    sqlx::query("SELECT * FROM Attempts WHERE status = ? ORDER BY attempt_date ASC")
        .bind(RunStatus::Pending.to_string())
        .fetch(pool)
        .then(async |row| {
            let row = row?;
            Ok(HgAttempt {
                attempt_number: row.try_get("attempt_number")?,
                run_id: row.try_get("run_id")?,
                attempt_date: row.try_get("attempt_date")?,
                user_defined_vars: serde_json::from_str(
                    row.try_get::<String, _>("user_defined_vars")?.as_str(),
                )
                .unwrap_or_default(),
                run_date: row.try_get("run_date")?,
                run_sequencer: row.try_get("run_sequencer")?,
                run_flowcellid: row.try_get("run_flowcellid")?,
                sample_sheet_adn_path: row.try_get("sample_sheet_adn_path")?,
                sample_sheet_arn_path: row.try_get("sample_sheet_arn_path")?,
                metadata_path: row.try_get("metadata_path")?,
                status: RunStatus::from_str(row.try_get::<String, _>("status")?.as_str())?,
                comment: row.try_get("comment")?,
            })
        })
        .collect::<Vec<Result<HgAttempt, RoutineError>>>()
        .await
}

async fn treat_attempt(
    run_result: Result<HgAttempt, RoutineError>,
    pool: &sqlx::SqlitePool,
) -> Result<(), RoutineError> {
    match run_result {
        Ok(attempt) => {
            println!(
                "Traitement de la tentative {} pour le run {}...",
                attempt.attempt_number, attempt.run_id
            );
            analysis::start_run_analysis(attempt.run_id, &pool).await?;
        }
        Err(e) => {
            eprintln!("Erreur lors de la récupération d'une tentative: {}", e);
        }
    }
    Ok(())
}

/// Fonction qui exécute la boucle de routine avec des points de contrôle pour l'annulation
async fn run_routine_loop(
    pool: sqlx::SqlitePool,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), RoutineError> {
    use std::time::Duration;

    loop {
        // Point de contrôle 1: Vérifier le signal d'arrêt au début de chaque itération
        if *shutdown_rx.borrow() {
            println!("Signal d'arrêt reçu, arrêt de la routine...");
            break;
        }

        println!("Routine: recherche de runs à traiter...");
        let pending_runs = find_pending_runs(&pool).await;
        if pending_runs.is_empty() {
            println!("Aucun run en attente.");
        } else {
            for run_result in pending_runs {
                match treat_attempt(run_result, &pool).await {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Erreur lors de l'exécution de la routine: {e}");
                    }
                }
            }
        }

        // Point de contrôle 2: Attendre avec possibilité d'interruption
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(300)) => {
                // Continue vers la prochaine itération
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    break;
                }
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), RoutineError> {
    use sqlx::sqlite::SqlitePool;
    use tokio::signal;

    println!("Connecting to database...");
    let pool = SqlitePool::connect("sqlite://mercure.db").await?;

    // Canal pour communiquer le signal d'arrêt
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Clone le pool pour la routine
    let routine_pool = pool.clone();

    // Lancer la routine dans une tâche séparée
    let routine_handle =
        tokio::spawn(async move { run_routine_loop(routine_pool, shutdown_rx).await });

    // Attendre le signal Ctrl+C
    match signal::ctrl_c().await {
        Ok(()) => {
            println!("Shutdown signal received, envoi du signal d'arrêt...");
            // Envoyer le signal d'arrêt à la routine
            let _ = shutdown_tx.send(true);

            // Attendre que la routine se termine proprement
            if let Err(e) = routine_handle.await {
                eprintln!("Erreur lors de l'arrêt de la routine: {}", e);
            }

            println!("Fermeture de la connexion à la base de données...");
            pool.close().await;
            println!("Connexion à la base de données fermée. Sortie.");
        }
        Err(err) => {
            eprintln!("Erreur lors de l'écoute du signal Ctrl+C: {}", err);
            return Err(RoutineError::SqlxError(sqlx::Error::Io(
                std::io::Error::new(std::io::ErrorKind::Other, format!("Signal error: {}", err)),
            )));
        }
    }

    Ok(())
}
