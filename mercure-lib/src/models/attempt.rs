use crate::config::get_mercure_config;
use crate::models::{FileIdentification, Form};
use crate::models::{ModelError, Run, RunStatus};
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use time::OffsetDateTime;

#[derive(wherror::Error, Debug)]
pub enum AttemptError {
    #[error("Aucun run n'est attaché à cette tentative")]
    NoRunAttached,
}

/// Created every time we attempt to analyze an HgRun.
///
/// Copies the editable data from HgRun at the time of attempt.
///
/// All of its fields are read-only except for comment and status.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Attempt {
    pub run_id: i64,
    pub attempt_number: u32,
    pub attempt_date: OffsetDateTime,

    pub user_defined_vars: HashMap<String, String>,

    pub form_identification: FileIdentification,

    /// One of the only two editable fields
    pub status: RunStatus,
    /// One of the only two editable fields
    pub comment: String,
}

impl Attempt {
    /// Crée une nouvelle tentative à partir d'une HgRun
    pub(super) async fn new_attempt(
        run_id: i64,
        current_commit_hash: &str,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        let run: Run = Run::get_run_from_id(run_id, pool).await?;

        let FileIdentification::GitBranchAndPath { file_path, .. } = run.form_identification else {
            return Err(anyhow::format_err!("Une nouvelle tentative ne peut être créée qu'à partir d'un formulaire défini par une branche et un chemin.").into());
        };

        let attempt_date = OffsetDateTime::now_utc();
        let user_defined_vars = serde_json::to_string(&run.user_defined_vars)?;

        sqlx::query(
            r#"
            INSERT INTO Attempts (run_id, attempt_number, attempt_date, user_defined_vars, commit_hash, form_path, status, comment)
            VALUES (?, ?, ?, ?, ?, ?, ?, '')
            "#,
        )
        .bind(run.run_id)
        .bind(run.attempt_count) // Pas d'incrémentation, le run que l'on tente d'analyser a déjà incrémenté son `attempt_count`
        .bind(&attempt_date)
        .bind(&user_defined_vars)
        .bind(&current_commit_hash)
        .bind(&file_path.display().to_string())
        .bind(run.status.to_string())
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Crée une tentative hypothétique à partir d'un HgRun (sans l'insérer en base)
    ///
    /// Utilisé pour prévisualiser les données d'une tentative avant de la créer réellement.
    /// Pratique pour l'API, uniformise l'environnement jinja2.
    pub fn get_hypothetic_attempt(run: &Run) -> Result<Attempt, ModelError> {
        Ok(Attempt {
            attempt_number: run.attempt_count + 1,
            run_id: run.run_id.ok_or(AttemptError::NoRunAttached)?,
            attempt_date: OffsetDateTime::now_utc(),
            user_defined_vars: run.user_defined_vars.clone(),
            form_identification: run.form_identification.clone(),
            status: RunStatus::Idle,
            comment: String::new(),
        })
    }

    pub async fn get_run(&self, pool: &SqlitePool) -> Result<Run, ModelError> {
        Ok(Run::get_run_from_id(self.run_id, pool).await?)
    }

    pub async fn get_form(&self) -> Result<Form, ModelError> {
        Form::get_form(self.form_identification.clone()).await
    }

    /// Auto-computed output directory for this attempt. Single definition point
    /// for the injected `outdir` jinja variable.
    pub fn outdir(&self) -> PathBuf {
        PathBuf::from(&get_mercure_config().logs_dir)
            .join(format!("run-{:0<8}", self.run_id))
            .join(format!("attempt-{:0<8}", self.attempt_number))
    }

    pub async fn render_script(&self) -> Result<String, ModelError> {
        let form = self.get_form().await?;
        let outdir = self.outdir().display().to_string();
        let ctx = form.jinja_context(&self.user_defined_vars, self.attempt_number, &outdir);
        Ok(form.render_template(&ctx).await?)
    }

    /// Récupère un HgAttempt à partir de son attempt_number et run_id
    pub async fn get_attempt_from_number(
        attempt_number: u32,
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
        let user_defined_vars: HashMap<String, String> =
            serde_json::from_str(row.try_get("user_defined_vars")?)?;

        let status = RunStatus::from_str(row.try_get::<String, _>("status")?.as_str())?;

        // Construire l'instance HgAttempt
        let attempt = Attempt {
            attempt_number,
            run_id: run_id,
            attempt_date: row.try_get("attempt_date")?,
            user_defined_vars,
            form_identification: FileIdentification::GitCommit {
                commit_hash: row.try_get("commit_hash")?,
                file_path: PathBuf::from_str(row.try_get("form_path")?)?,
            },
            status,
            comment: row.try_get("comment")?,
        };

        Ok(attempt)
    }

    pub async fn get_pending_runs(pool: &SqlitePool) -> Result<Vec<Self>, ModelError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM Attempts WHERE status = 'Pending' ORDER BY attempt_number DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                Some(Attempt {
                    attempt_number: row.try_get("attempt_number").ok()?,
                    run_id: row.try_get("run_id").ok()?,
                    attempt_date: row.try_get("attempt_date").ok()?,
                    user_defined_vars: serde_json::from_str(row.try_get("attempt_number").ok()?)
                        .ok()?,
                    form_identification: FileIdentification::GitCommit {
                        commit_hash: row.try_get("commit_hash").ok()?,
                        file_path: PathBuf::from_str(row.try_get("form_path").ok()?).ok()?,
                    },
                    status: RunStatus::Pending,
                    comment: row.try_get("comment").ok()?,
                })
            })
            .collect())
    }

    /// Récupère toutes les tentatives pour un run_id
    pub async fn list_attempts_for_run(
        run_id: i64,
        pool: &SqlitePool,
    ) -> Result<Vec<Self>, ModelError> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM Attempts WHERE run_id = ? ORDER BY attempt_number DESC
            "#,
        )
        .bind(run_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                // Désérialiser les variables définies par l'utilisateur
                let user_defined_vars: HashMap<String, String> =
                    serde_json::from_str(row.try_get("user_defined_vars").ok()?).ok()?;

                // Parser le statut
                let status_str: String = row.try_get("status").ok()?;
                let status = RunStatus::from_str(status_str.as_str()).ok()?;
                Some(Attempt {
                    attempt_number: row.try_get("attempt_number").ok()?,
                    run_id: row.try_get("run_id").ok()?,
                    attempt_date: row.try_get("attempt_date").ok()?,
                    user_defined_vars,
                    form_identification: FileIdentification::GitCommit {
                        commit_hash: row.try_get("commit_hash").ok()?,
                        file_path: PathBuf::from_str(row.try_get("form_path").ok()?).ok()?,
                    },
                    status,
                    comment: row.try_get("comment").ok()?,
                })
            })
            .collect())
    }

    pub(super) async fn start_run(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        let run = Run::get_run_from_id(run_id, pool).await?;
        sqlx::query(r#"UPDATE Attempts SET status = ? WHERE run_id = ? AND attempt_number = ?"#)
            .bind("Running")
            .bind(run_id)
            .bind(run.attempt_count)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// La tentative `attempt_number` s'est terminée avec succès.
    pub(super) async fn complete_success(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        let run = Run::get_run_from_id(run_id, pool).await?;
        sqlx::query(
            r#"
            UPDATE Attempts SET status = ? WHERE run_id = ? AND attempt_number = ?
            "#,
        )
        .bind("Success")
        .bind(run_id)
        .bind(run.attempt_count)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// La tentative `attempt_number` a échoué.
    pub(super) async fn complete_failure(
        run_id: i64,
        reason: &str,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        let run = Run::get_run_from_id(run_id, pool).await?;
        sqlx::query(
            r#"
            UPDATE Attempts SET status = ? WHERE run_id = ? AND attempt_number = ?
            "#,
        )
        .bind(format!("Failure:{}", reason))
        .bind(run_id)
        .bind(run.attempt_count)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Met à jour le commentaire d'une tentative
    pub(super) async fn update_comment(
        run_id: i64,
        attempt_number: u32,
        new_comment: &str,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        sqlx::query(
            r#"
            UPDATE Attempts SET comment = ? WHERE attempt_number = ? AND run_id = ?
            "#,
        )
        .bind(new_comment)
        .bind(attempt_number)
        .bind(run_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
