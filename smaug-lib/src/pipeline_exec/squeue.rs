use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::Instant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobStatus {
    pub job_id: String,
    pub job_name: String,
    pub state: String,
    pub elapsed: String,
    pub time_limit: String,
    pub nodes: String,
    pub nodelist: String,
    pub reason: String,
}

struct CacheEntry {
    jobs: Vec<JobStatus>,
    fetched_at: Instant,
}

pub struct SqueueCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    ttl: Duration,
}

impl SqueueCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub async fn get(&self, job_name: &str) -> Option<Vec<JobStatus>> {
        let job_name = job_name.trim();
        if job_name.is_empty() {
            return None;
        }

        let cached = {
            let entries = self.entries.read().await;
            entries
                .get(job_name)
                .filter(|entry| entry.fetched_at.elapsed() < self.ttl)
                .map(|entry| entry.jobs.clone())
        };

        if let Some(jobs) = cached {
            return Some(jobs);
        }

        let jobs = fetch_squeue_from_slurm(job_name).await?;

        let mut entries = self.entries.write().await;
        entries.retain(|_, entry| entry.fetched_at.elapsed() < self.ttl);
        entries.insert(
            job_name.to_string(),
            CacheEntry {
                jobs: jobs.clone(),
                fetched_at: Instant::now(),
            },
        );

        Some(jobs)
    }

    pub async fn invalidate(&self, job_name: &str) {
        self.entries.write().await.remove(job_name);
    }
}

pub async fn fetch_squeue_from_slurm(job_name: &str) -> Option<Vec<JobStatus>> {
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new("squeue")
            .args([
                "-h",
                "-n",
                job_name,
                "-o",
                "%.20i|%.80j|%.20T|%.20M|%.20l|%.10D|%.80N|%.80r",
            ])
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;

    Some(
        stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(parse_job_line)
            .collect(),
    )
}

fn parse_job_line(line: &str) -> Option<JobStatus> {
    let mut parts = line.splitn(8, '|');

    Some(JobStatus {
        job_id: parts.next()?.trim().to_string(),
        job_name: parts.next()?.trim().to_string(),
        state: parts.next()?.trim().to_string(),
        elapsed: parts.next()?.trim().to_string(),
        time_limit: parts.next()?.trim().to_string(),
        nodes: parts.next()?.trim().to_string(),
        nodelist: parts.next()?.trim().to_string(),
        reason: parts.next().unwrap_or_default().trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pipe_separated_squeue_line() {
        let job =
            parse_job_line("12345|pipeline-job|RUNNING|00:12:34|1-00:00:00|2|node[01-02]|None")
                .unwrap();

        assert_eq!(job.job_id, "12345");
        assert_eq!(job.job_name, "pipeline-job");
        assert_eq!(job.state, "RUNNING");
        assert_eq!(job.elapsed, "00:12:34");
        assert_eq!(job.time_limit, "1-00:00:00");
        assert_eq!(job.nodes, "2");
        assert_eq!(job.nodelist, "node[01-02]");
        assert_eq!(job.reason, "None");
    }

    #[test]
    fn rejects_incomplete_squeue_line() {
        assert!(parse_job_line("12345|pipeline-job|RUNNING").is_none());
    }
}
