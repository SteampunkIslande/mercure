use crate::models::ModelError;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, FromRow)]
pub struct RunProgress {
    pub run_id: i64,
    pub attempt_number: i64,
    pub current_step_string: Option<String>,
    pub current_progress: Option<i64>,
    pub last_output_line: Option<String>,
    pub done_jobs_count: Option<i64>,
    pub slurm_job_name: Option<String>,
    pub log_available: bool,
    pub updated_at: OffsetDateTime,
}

impl RunProgress {
    pub fn new(run_id: i64, attempt_number: u32) -> Self {
        Self {
            run_id,
            attempt_number: i64::from(attempt_number),
            current_step_string: None,
            current_progress: None,
            last_output_line: None,
            done_jobs_count: None,
            slurm_job_name: None,
            log_available: false,
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    pub async fn get(
        run_id: i64,
        attempt_number: u32,
        pool: &SqlitePool,
    ) -> Result<Option<Self>, ModelError> {
        let progress = sqlx::query_as::<_, Self>(
            r#"
            SELECT run_id, attempt_number, current_step_string, current_progress,
                   last_output_line, done_jobs_count, slurm_job_name, log_available, updated_at
            FROM RunProgress
            WHERE run_id = ? AND attempt_number = ?
            "#,
        )
        .bind(run_id)
        .bind(i64::from(attempt_number))
        .fetch_optional(pool)
        .await?;

        Ok(progress)
    }

    pub async fn get_or_default(
        run_id: i64,
        attempt_number: u32,
        pool: &SqlitePool,
    ) -> Result<Self, ModelError> {
        Ok(Self::get(run_id, attempt_number, pool)
            .await?
            .unwrap_or_else(|| Self::new(run_id, attempt_number)))
    }

    pub async fn upsert(&self, pool: &SqlitePool) -> Result<(), ModelError> {
        sqlx::query(
            r#"
            INSERT INTO RunProgress (
                run_id, attempt_number, current_step_string, current_progress,
                last_output_line, done_jobs_count, slurm_job_name, log_available, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(run_id, attempt_number) DO UPDATE SET
                current_step_string = excluded.current_step_string,
                current_progress = excluded.current_progress,
                last_output_line = excluded.last_output_line,
                done_jobs_count = excluded.done_jobs_count,
                slurm_job_name = excluded.slurm_job_name,
                log_available = excluded.log_available,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(self.run_id)
        .bind(self.attempt_number)
        .bind(&self.current_step_string)
        .bind(self.current_progress)
        .bind(&self.last_output_line)
        .bind(self.done_jobs_count)
        .bind(&self.slurm_job_name)
        .bind(self.log_available)
        .bind(OffsetDateTime::now_utc())
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    #[tokio::test]
    async fn upsert_and_read_roundtrip() {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(false);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();

        crate::db::run_migrations(&pool).await.unwrap();

        let mut progress = RunProgress::new(1, 2);
        progress.log_available = true;
        progress.current_step_string = Some("step".into());
        progress.current_progress = Some(42);
        progress.last_output_line = Some("line".into());
        progress.done_jobs_count = Some(3);
        progress.slurm_job_name = Some("job".into());

        progress.upsert(&pool).await.unwrap();

        let loaded = RunProgress::get(1, 2, &pool).await.unwrap().unwrap();

        assert_eq!(loaded.run_id, 1);
        assert_eq!(loaded.attempt_number, 2);
        assert_eq!(loaded.current_step_string.as_deref(), Some("step"));
        assert_eq!(loaded.current_progress, Some(42));
        assert_eq!(loaded.last_output_line.as_deref(), Some("line"));
        assert_eq!(loaded.done_jobs_count, Some(3));
        assert_eq!(loaded.slurm_job_name.as_deref(), Some("job"));
        assert!(loaded.log_available);

        let missing = RunProgress::get_or_default(2, 1, &pool).await.unwrap();
        assert!(!missing.log_available);
        assert_eq!(missing.run_id, 2);
        assert_eq!(missing.attempt_number, 1);
    }
}
