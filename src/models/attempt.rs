use crate::models::{HgRun, ModelError, RunStatus};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use time::OffsetDateTime;

/// Created every time we attempt to analyze an HgRun.
/// Copies the editable data from HgRun at the time of attempt.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HgAttempt {
    pub attempt_id: i64,
    pub run_id: i64,
    pub attempt_date: String,
    pub user_defined_vars: HashMap<String, String>,
    pub run_date: String,
    pub run_sequencer: String,
    pub run_flowcellid: String,
    pub sample_sheet_adn_path: String,
    pub sample_sheet_arn_path: String,
    pub metadata_path: String,
    pub status: RunStatus,
    pub comment: String,
}

impl HgAttempt {
    /// Crée une nouvelle tentative à partir d'une HgRun
    pub async fn new_attempt(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        let run: HgRun = HgRun::get_run_from_id(run_id, pool).await?;

        let attempt_date = OffsetDateTime::now_utc().to_string();
        let user_defined_vars_json = serde_json::to_string(&run.user_defined_vars)
            .map_err(|e| ModelError::FormError(format!("Erreur de sérialisation JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO Attempts (run_id, attempt_date, user_defined_vars, run_date, run_sequencer, run_flowcellid, sample_sheet_adn_path, sample_sheet_arn_path, metadata_path, status, comment)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '')
            "#,
        )
        .bind(run.run_id)
        .bind(&attempt_date)
        .bind(&user_defined_vars_json)
        .bind(&run.run_date)
        .bind(&run.run_sequencer)
        .bind(&run.run_flowcellid)
        .bind(&run.sample_sheet_adn_path)
        .bind(&run.sample_sheet_arn_path)
        .bind(&run.metadata_path)
        .bind(serde_json::to_string(&run.status).unwrap_or_else(|_| "Idle".to_string()))
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Instancie un HgAttempt à partir de son attempt_id
    pub async fn get_attempt_from_id(
        attempt_id: i64,
        pool: &SqlitePool,
    ) -> Result<Self, ModelError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM Attempts WHERE attempt_id = ?
            "#,
        )
        .bind(attempt_id)
        .fetch_one(pool)
        .await?;

        // Désérialiser les variables définies par l'utilisateur
        let user_defined_vars_json: String = row.try_get("user_defined_vars")?;
        let user_defined_vars: HashMap<String, String> =
            serde_json::from_str(&user_defined_vars_json).map_err(|e| {
                ModelError::FormError(format!("Erreur de désérialisation JSON: {}", e))
            })?;

        // Parser le statut
        let status_str: String = row.try_get("status")?;
        let status = match status_str.as_str() {
            "Idle" => RunStatus::Idle,
            "Pending" => RunStatus::Pending,
            "Running" => RunStatus::Running,
            "Success" => RunStatus::Success,
            s if s.starts_with("Failure:") => RunStatus::Failure(s[8..].to_string()),
            _ => RunStatus::Idle, // Défaut en cas d'inconnu
        };

        // Construire l'instance HgAttempt
        let attempt = HgAttempt {
            attempt_id,
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
    pub async fn get_attempts_for_run(
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

        let mut attempts = Vec::new();
        for row in rows {
            // Désérialiser les variables définies par l'utilisateur
            let user_defined_vars_json: String = row.try_get("user_defined_vars")?;
            let user_defined_vars: HashMap<String, String> =
                serde_json::from_str(&user_defined_vars_json).map_err(|e| {
                    ModelError::FormError(format!("Erreur de désérialisation JSON: {}", e))
                })?;

            // Parser le statut
            let status_str: String = row.try_get("status")?;
            let status = match status_str.as_str() {
                "Idle" => RunStatus::Idle,
                "Pending" => RunStatus::Pending,
                "Running" => RunStatus::Running,
                "Success" => RunStatus::Success,
                s if s.starts_with("Failure:") => RunStatus::Failure(s[8..].to_string()),
                _ => RunStatus::Idle, // Défaut en cas d'inconnu
            };

            let attempt = HgAttempt {
                attempt_id: row.try_get("attempt_id")?,
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
            attempts.push(attempt);
        }

        Ok(attempts)
    }

    /// Démarre toutes les tentatives pour un run : Pending -> Running
    pub async fn start_attempts_for_run(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        sqlx::query(
            r#"
            UPDATE Attempts SET status = ? WHERE run_id = ? AND status = ?
            "#,
        )
        .bind("Running")
        .bind(run_id)
        .bind("Pending")
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Termine toutes les tentatives avec succès : Running -> Success
    pub async fn complete_attempts_success_for_run(
        run_id: i64,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        sqlx::query(
            r#"
            UPDATE Attempts SET status = ? WHERE run_id = ? AND status = ?
            "#,
        )
        .bind("Success")
        .bind(run_id)
        .bind("Running")
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Termine toutes les tentatives avec échec : Running -> Failure
    pub async fn complete_attempts_failure_for_run(
        run_id: i64,
        reason: String,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        sqlx::query(
            r#"
            UPDATE Attempts SET status = ? WHERE run_id = ? AND status = ?
            "#,
        )
        .bind(format!("Failure:{}", reason))
        .bind(run_id)
        .bind("Running")
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Met à jour le commentaire d'une tentative
    pub async fn update_comment(
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
