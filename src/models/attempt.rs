use crate::models::{HgRun, ModelError, RunStatus};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::str::FromStr;
use time::OffsetDateTime;

/// Created every time we attempt to analyze an HgRun.
///
/// Copies the editable data from HgRun at the time of attempt.
///
/// All of its fields are read-only except for comment and status.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HgAttempt {
    pub attempt_number: i64,
    pub run_id: i64,
    pub attempt_date: String,
    pub user_defined_vars: HashMap<String, String>,
    pub run_date: String,
    pub run_sequencer: String,
    pub run_flowcellid: String,
    pub sample_sheet_adn_path: String,
    pub sample_sheet_arn_path: String,
    pub metadata_path: String,

    /// One of the only two editable fields
    pub status: RunStatus,
    /// One of the only two editable fields
    pub comment: String,
}

impl HgAttempt {
    /// Crée une nouvelle tentative à partir d'une HgRun
    pub(super) async fn new_attempt(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        let run: HgRun = HgRun::get_run_from_id(run_id, pool).await?;

        let attempt_date = OffsetDateTime::now_utc().to_string();
        let user_defined_vars_json = serde_json::to_string(&run.user_defined_vars)
            .map_err(|e| ModelError::FormError(format!("Erreur de sérialisation JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO Attempts (attempt_number, run_id, attempt_date, user_defined_vars, run_date, run_sequencer, run_flowcellid, sample_sheet_adn_path, sample_sheet_arn_path, metadata_path, status, comment)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '')
            "#,
        )
        .bind(run.attempt_count) // Pas d'incrémentation, le run que l'on tente d'analyser a déjà incrémenté son `attempt_count`
        .bind(run.run_id)
        .bind(&attempt_date)
        .bind(&user_defined_vars_json)
        .bind(&run.run_date)
        .bind(&run.run_sequencer)
        .bind(&run.run_flowcellid)
        .bind(&run.sample_sheet_adn_path)
        .bind(&run.sample_sheet_arn_path)
        .bind(&run.metadata_path)
        .bind(&run.status.to_string())
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Récupère un HgAttempt à partir de son attempt_number et run_id
    pub async fn get_attempt_from_number(
        attempt_number: i64,
        run_id: i64,
        pool: &SqlitePool,
    ) -> Result<Self, ModelError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM Attempts WHERE attempt_number = ? AND run_id = ?
            "#,
        )
        .bind(attempt_number)
        .bind(run_id)
        .fetch_one(pool)
        .await?;

        // Désérialiser les variables définies par l'utilisateur
        let user_defined_vars_json: String = row.try_get("user_defined_vars")?;
        let user_defined_vars: HashMap<String, String> =
            serde_json::from_str(&user_defined_vars_json).map_err(|e| {
                ModelError::FormError(format!("Erreur de désérialisation JSON: {}", e))
            })?;

        let status = RunStatus::from_str(row.try_get::<String, _>("status")?.as_str())
            .unwrap_or(RunStatus::Idle);

        // Construire l'instance HgAttempt
        let attempt = HgAttempt {
            attempt_number,
            run_id: row.try_get("run_id")?,
            attempt_date: row.try_get("attempt_date")?,
            user_defined_vars,
            run_date: row.try_get("run_date")?,
            run_sequencer: row.try_get("run_sequencer")?,
            run_flowcellid: row.try_get("run_flowcellid")?,
            sample_sheet_adn_path: row.try_get("sample_sheet_adn_path")?,
            sample_sheet_arn_path: row.try_get("sample_sheet_arn_path")?,
            metadata_path: row.try_get("metadata_path")?,
            status,
            comment: row.try_get("comment")?,
        };

        Ok(attempt)
    }

    /// Récupère toutes les tentatives pour un run_id
    pub async fn list_attempts_for_run(
        run_id: i64,
        pool: &SqlitePool,
    ) -> Result<Vec<Self>, ModelError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM Attempts WHERE run_id = ?
            "#,
        )
        .bind(run_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                // Désérialiser les variables définies par l'utilisateur
                let user_defined_vars_json: String = row.try_get("user_defined_vars").ok()?;
                let user_defined_vars: HashMap<String, String> =
                    serde_json::from_str(&user_defined_vars_json)
                        .map_err(|e| {
                            ModelError::FormError(format!("Erreur de désérialisation JSON: {}", e))
                        })
                        .ok()?;

                // Parser le statut
                let status_str: String = row.try_get("status").ok()?;
                let status = RunStatus::from_str(status_str.as_str()).ok()?;
                Some(HgAttempt {
                    attempt_number: row.try_get("attempt_number").ok()?,
                    run_id: row.try_get("run_id").ok()?,
                    attempt_date: row.try_get("attempt_date").ok()?,
                    user_defined_vars,
                    run_date: row.try_get("run_date").ok()?,
                    run_sequencer: row.try_get("run_sequencer").ok()?,
                    run_flowcellid: row.try_get("run_flowcellid").ok()?,
                    sample_sheet_adn_path: row.try_get("sample_sheet_adn_path").ok()?,
                    sample_sheet_arn_path: row.try_get("sample_sheet_arn_path").ok()?,
                    metadata_path: row.try_get("metadata_path").ok()?,
                    status,
                    comment: row.try_get("comment").ok()?,
                })
            })
            .collect())
    }

    pub(super) async fn start_run(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        let run = HgRun::get_run_from_id(run_id, pool).await?;
        sqlx::query(r#"UPDATE Attempts SET status = ? WHERE run_id = ? AND attempt_number = ?"#)
            .bind("Running")
            .bind(run_id)
            .bind(run.attempt_count)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// La tentative `attempt_number` s'est terminée avec succès.
    pub(super) async fn complete_success(
        run_id: i64,
        attempt_number: i64,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        sqlx::query(
            r#"
            UPDATE Attempts SET status = ? WHERE run_id = ? AND attempt_number = ?
            "#,
        )
        .bind("Success")
        .bind(run_id)
        .bind(attempt_number)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// La tentative `attempt_number` a échoué.
    pub(super) async fn complete_failure(
        run_id: i64,
        attempt_number: i64,
        reason: String,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        sqlx::query(
            r#"
            UPDATE Attempts SET status = ? WHERE run_id = ? AND attempt_number = ?
            "#,
        )
        .bind(format!("Failure:{}", reason))
        .bind(run_id)
        .bind(attempt_number)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Met à jour le commentaire d'une tentative
    pub(super) async fn update_comment(
        attempt_id: i64,
        new_comment: String,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        sqlx::query(
            r#"
            UPDATE Attempts SET comment = ? WHERE attempt_id = ?
            "#,
        )
        .bind(&new_comment)
        .bind(attempt_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
