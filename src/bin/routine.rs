use std::str::FromStr;

use futures::StreamExt;
use sqlx;
use sqlx::Row;
use thiserror::Error;

use mercure::models::HgAttempt;
use mercure::models::InvalidRunStatusError;
use mercure::models::RunStatus;

#[derive(Error, Debug)]
enum RoutineError {
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error(transparent)]
    InvalidRunStatusError(#[from] InvalidRunStatusError),
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

#[tokio::main]
async fn main() -> Result<(), RoutineError> {
    use sqlx::sqlite::SqlitePool;
    use std::time::Duration;
    use tokio::signal;

    println!("Connecting to database...");
    let pool = SqlitePool::connect("sqlite://mercure.db").await?;

    loop {
        println!("Routine: recherche de runs à traiter...");

        let pending_runs = find_pending_runs(&pool).await;
        if pending_runs.is_empty() {
            println!("Aucun run en attente.");
        } else {
            for run_result in pending_runs {
                match run_result {
                    Ok(attempt) => {
                        println!(
                            "Traitement de la tentative {} pour le run {}...",
                            attempt.attempt_number, attempt.run_id
                        );
                    }
                    Err(e) => {
                        eprintln!("Erreur lors de la récupération d'une tentative: {}", e);
                    }
                }
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(300)) => {
                // Continue to next iteration
            }
            _ = signal::ctrl_c() => {
                println!("Shutdown signal received, closing pool...");
                pool.close().await;
                println!("Pool closed. Exiting.");
                break;
            }
        }
    }
    Ok(())
}
