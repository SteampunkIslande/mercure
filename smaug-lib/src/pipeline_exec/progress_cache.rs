use crate::models::RunProgress;
use std::collections::HashMap;
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct RunProgressCache {
    entries: Arc<RwLock<HashMap<(i64, u32), RunProgress>>>,
}

impl RunProgressCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_or_default(&self, run_id: i64, attempt_number: u32) -> RunProgress {
        self.entries
            .read()
            .await
            .get(&(run_id, attempt_number))
            .cloned()
            .unwrap_or_else(|| RunProgress::new(run_id, attempt_number))
    }

    pub async fn upsert(&self, mut progress: RunProgress) {
        progress.updated_at = OffsetDateTime::now_utc();
        let key = (progress.run_id, progress.attempt_number);
        self.entries.write().await.insert(key, progress);
    }

    pub async fn remove(&self, run_id: i64, attempt_number: u32) {
        self.entries.write().await.remove(&(run_id, attempt_number));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn inserts_and_reads_progress() {
        let cache = RunProgressCache::new();

        let missing = cache.get_or_default(1, 1).await;
        assert!(!missing.log_available);
        assert_eq!(missing.current_progress, None);

        let mut progress = RunProgress::new(1, 1);
        progress.log_available = true;
        progress.current_step_string = Some("step".into());
        progress.current_progress = Some(42);
        progress.done_jobs_count = Some(3);
        progress.slurm_job_name = Some("job".into());

        cache.upsert(progress).await;

        let loaded = cache.get_or_default(1, 1).await;

        assert!(loaded.log_available);
        assert_eq!(loaded.current_step_string.as_deref(), Some("step"));
        assert_eq!(loaded.current_progress, Some(42));
        assert_eq!(loaded.done_jobs_count, Some(3));
        assert_eq!(loaded.slurm_job_name.as_deref(), Some("job"));
    }

    #[tokio::test]
    async fn attempts_are_cached_separately() {
        let cache = RunProgressCache::new();

        let mut first = RunProgress::new(1, 1);
        first.current_step_string = Some("first".into());
        cache.upsert(first).await;

        let mut second = RunProgress::new(1, 2);
        second.current_step_string = Some("second".into());
        cache.upsert(second).await;

        assert_eq!(
            cache
                .get_or_default(1, 1)
                .await
                .current_step_string
                .as_deref(),
            Some("first")
        );
        assert_eq!(
            cache
                .get_or_default(1, 2)
                .await
                .current_step_string
                .as_deref(),
            Some("second")
        );
    }

    #[tokio::test]
    async fn remove_deletes_the_entry() {
        let cache = RunProgressCache::new();
        cache.upsert(RunProgress::new(1, 1)).await;
        cache.remove(1, 1).await;

        assert!(!cache.get_or_default(1, 1).await.log_available);
    }
}
