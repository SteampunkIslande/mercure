use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use log::warn;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::sync::Mutex;

use crate::models::Attempt;

/// Incrementally records the snakemake jobs of an attempt in a sqlite report.
///
/// The report is written while the analysis runs, from the lines snakemake
/// writes on stderr. It lives next to the attempt's output so that it survives
/// the run and can be inspected afterwards.
#[derive(Clone)]
pub struct RunReport {
    inner: Arc<RunReportInner>,
}

struct RunReportInner {
    pool: SqlitePool,
    outdir: PathBuf,
    state: Mutex<ReportState>,
}

#[derive(Default)]
struct ReportState {
    /// UUID of the current snakemake workflow, emitted as `SLURM run ID: <uuid>`.
    slurm_job_name: Option<String>,
    /// Job block currently being printed by snakemake.
    block: Option<JobBlock>,
    /// Set after an `Error in rule ...:` header: its indented properties describe
    /// an already reported failure and must not be merged into another job.
    ignore_properties: bool,
    /// Database row ids of the already inserted jobs, keyed by (workflow, jobid).
    row_ids: HashMap<(String, i64), i64>,
}

struct JobBlock {
    inputs: Vec<String>,
    log_paths: Vec<String>,
    wildcards: Vec<String>,
    jobid: Option<i64>,
    field: Option<BlockField>,
}

#[derive(Clone, Copy)]
enum BlockField {
    Inputs,
    Logs,
}

impl JobBlock {
    fn new() -> Self {
        Self {
            inputs: Vec::new(),
            log_paths: Vec::new(),
            wildcards: Vec::new(),
            jobid: None,
            field: None,
        }
    }
}

impl RunReport {
    /// Creates the report database in the attempt output directory and prepares
    /// its schema.
    pub async fn create(attempt: &Attempt) -> Result<Self> {
        let path = attempt.run_report_path();
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .with_context(|| {
                format!(
                    "Impossible de créer le rapport d'exécution {}",
                    path.display()
                )
            })?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS snakemake (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                slurm_job_name TEXT,
                slurm_job_id TEXT,
                snakemake_job_id INTEGER NOT NULL,
                log_paths TEXT NOT NULL DEFAULT '',
                wildcards TEXT NOT NULL DEFAULT '',
                input_paths TEXT NOT NULL DEFAULT '',
                input_sizes TEXT NOT NULL DEFAULT ''
            )
            "#,
        )
        .execute(&pool)
        .await
        .context("Impossible de créer la table `snakemake` du rapport d'exécution")?;

        Ok(Self {
            inner: Arc::new(RunReportInner {
                pool,
                outdir: attempt.outdir(),
                state: Mutex::new(ReportState::default()),
            }),
        })
    }

    /// Consumes one stderr line. Failures are logged and never stop the analysis.
    pub async fn observe(&self, raw_line: &str) {
        let line = strip_ansi(raw_line);
        if line.trim().is_empty() {
            return;
        }

        let mut state = self.inner.state.lock().await;
        self.observe_locked(&mut state, &line).await;
    }

    /// Persists the last job block once snakemake has stopped writing.
    pub async fn finish(&self) {
        let mut state = self.inner.state.lock().await;
        if let Some(block) = state.block.take() {
            let slurm_job_name = state.slurm_job_name.clone();
            if let Err(e) = self.persist(&mut state, block, None, slurm_job_name).await {
                warn!("Impossible d'écrire le rapport d'exécution: {e:#}");
            }
        }
    }

    async fn observe_locked(&self, state: &mut ReportState, line: &str) {
        let trimmed = line.trim();

        if let Some(job_name) = parse_slurm_run_id(trimmed) {
            state.slurm_job_name = Some(job_name);
            return;
        }

        if let Some(submission) = parse_slurm_submission(trimmed) {
            if let Some(block) = state.block.as_ref()
                && block.jobid == Some(submission.snakemake_job_id)
            {
                let block = state.block.take().expect("block checked above");
                if let Err(e) = self
                    .persist(state, block, Some(submission), state.slurm_job_name.clone())
                    .await
                {
                    warn!("Impossible d'écrire le rapport d'exécution: {e:#}");
                }
            } else {
                // The job block was not seen: still record what the submission line
                // gives us, with the snakemake job id it names.
                let block = JobBlock {
                    jobid: Some(submission.snakemake_job_id),
                    ..JobBlock::new()
                };
                if let Err(e) = self
                    .persist(state, block, Some(submission), state.slurm_job_name.clone())
                    .await
                {
                    warn!("Impossible d'écrire le rapport d'exécution: {e:#}");
                }
            }
            return;
        }

        if is_error_rule_header(trimmed) {
            // The failed job's block was open; flush it before ignoring the
            // duplicated metadata printed by the error report.
            if let Some(block) = state.block.take()
                && let Err(e) = self
                    .persist(state, block, None, state.slurm_job_name.clone())
                    .await
            {
                warn!("Impossible d'écrire le rapport d'exécution: {e:#}");
            }
            state.ignore_properties = true;
            return;
        }

        if parse_rule_header(trimmed) {
            if let Some(block) = state.block.take() {
                // A block without a submission line is a locally executed job.
                if let Err(e) = self
                    .persist(state, block, None, state.slurm_job_name.clone())
                    .await
                {
                    warn!("Impossible d'écrire le rapport d'exécution: {e:#}");
                }
            }
            state.block = Some(JobBlock::new());
            state.ignore_properties = false;
            return;
        }

        if state.ignore_properties {
            return;
        }

        let Some(block) = state.block.as_mut() else {
            return;
        };

        if let Some((key, value)) = parse_block_property(line) {
            match key {
                "input" => {
                    block.field = Some(BlockField::Inputs);
                    append_values(&mut block.inputs, value);
                }
                "log" => {
                    block.field = Some(BlockField::Logs);
                    append_values(&mut block.log_paths, value);
                }
                "wildcards" => {
                    block.field = None;
                    block.wildcards.extend(wildcard_values(value));
                }
                "jobid" => {
                    block.field = None;
                    block.jobid = value.trim().parse().ok();
                }
                _ => {
                    block.field = None;
                }
            }
            return;
        }

        // Continuation line of a multi-line `input:` or `log:` property.
        if is_indented(line)
            && let Some(field) = block.field
        {
            match field {
                BlockField::Inputs => append_values(&mut block.inputs, line),
                BlockField::Logs => append_values(&mut block.log_paths, line),
            }
        }
    }

    async fn persist(
        &self,
        state: &mut ReportState,
        block: JobBlock,
        submission: Option<SlurmSubmission>,
        slurm_job_name: Option<String>,
    ) -> Result<()> {
        let Some(snakemake_job_id) = block.jobid else {
            return Ok(());
        };

        let key = (slurm_job_name.clone().unwrap_or_default(), snakemake_job_id);

        let mut log_paths = block
            .log_paths
            .iter()
            .map(|path| absolutize(&self.inner.outdir, path))
            .collect::<Vec<_>>();
        let mut slurm_job_id = None;
        if let Some(submission) = submission {
            slurm_job_id = Some(submission.slurm_job_id);
            if let Some(path) = submission.log_path {
                log_paths.push(absolutize(&self.inner.outdir, &path));
            }
        }
        log_paths.sort();
        log_paths.dedup();

        let input_paths = block
            .inputs
            .iter()
            .map(|path| absolutize(&self.inner.outdir, path))
            .collect::<Vec<_>>();
        let mut input_sizes = Vec::with_capacity(input_paths.len());
        for path in &input_paths {
            let size = tokio::fs::metadata(path)
                .await
                .map(|meta| meta.len().to_string())
                .unwrap_or_else(|_| "0".to_string());
            input_sizes.push(size);
        }

        let log_paths = log_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(",");
        let wildcards = block.wildcards.join(",");
        let input_paths = input_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(",");
        let input_sizes = input_sizes.join(",");

        if let Some(row_id) = state.row_ids.get(&key).copied() {
            sqlx::query(
                r#"
                UPDATE snakemake
                SET slurm_job_name = ?, slurm_job_id = ?, log_paths = ?,
                    wildcards = ?, input_paths = ?, input_sizes = ?
                WHERE id = ?
                "#,
            )
            .bind(&key.0)
            .bind(&slurm_job_id)
            .bind(&log_paths)
            .bind(&wildcards)
            .bind(&input_paths)
            .bind(&input_sizes)
            .bind(row_id)
            .execute(&self.inner.pool)
            .await
            .context("Impossible de mettre à jour le rapport d'exécution")?;
        } else {
            let result = sqlx::query(
                r#"
                INSERT INTO snakemake
                    (slurm_job_name, slurm_job_id, snakemake_job_id, log_paths,
                     wildcards, input_paths, input_sizes)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&key.0)
            .bind(&slurm_job_id)
            .bind(snakemake_job_id)
            .bind(&log_paths)
            .bind(&wildcards)
            .bind(&input_paths)
            .bind(&input_sizes)
            .execute(&self.inner.pool)
            .await
            .context("Impossible d'insérer dans le rapport d'exécution")?;
            state.row_ids.insert(key, result.last_insert_rowid());
        }

        Ok(())
    }
}

struct SlurmSubmission {
    snakemake_job_id: i64,
    slurm_job_id: String,
    log_path: Option<String>,
}

fn parse_slurm_run_id(line: &str) -> Option<String> {
    let (_, value) = line.split_once("SLURM run ID:")?;
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Parses `Job <snakemake_job_id> has been submitted with SLURM jobid <id> (log: <path>)`.
fn parse_slurm_submission(line: &str) -> Option<SlurmSubmission> {
    let rest = line.strip_prefix("Job ")?;
    let (snakemake_job_id, rest) = rest.split_once(" has been submitted with SLURM jobid ")?;
    let (slurm_job_id, rest) = match rest.split_once(" (log:") {
        Some((id, rest)) => (id, Some(rest)),
        None => (rest, None),
    };
    let slurm_job_id = slurm_job_id.trim();
    if slurm_job_id.is_empty() {
        return None;
    }
    let log_path = rest.and_then(|rest| {
        let path = rest.trim();
        let path = path.strip_suffix('.').unwrap_or(path);
        let path = path.strip_suffix(')').unwrap_or(path);
        let path = path.trim();
        (!path.is_empty()).then(|| path.to_string())
    });

    Some(SlurmSubmission {
        snakemake_job_id: snakemake_job_id.trim().parse().ok()?,
        slurm_job_id: slurm_job_id.to_string(),
        log_path,
    })
}

fn parse_rule_header(line: &str) -> bool {
    let Some(line) = line.strip_suffix(':') else {
        return false;
    };
    let Some(rule_name) = line
        .strip_prefix("localrule ")
        .or_else(|| line.strip_prefix("rule "))
    else {
        return false;
    };
    !rule_name.trim().is_empty()
}

fn is_error_rule_header(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("Error in rule ") else {
        return false;
    };
    rest.strip_suffix(':')
        .is_some_and(|name| !name.trim().is_empty())
}

fn parse_block_property(line: &str) -> Option<(&str, &str)> {
    if !is_indented(line) {
        return None;
    }
    let (key, value) = line.trim_start().split_once(':')?;
    let key = key.trim();
    if key.is_empty() || key.contains(' ') {
        return None;
    }
    Some((key, value.trim()))
}

fn is_indented(line: &str) -> bool {
    line.starts_with(' ') || line.starts_with('\t')
}

fn append_values(values: &mut Vec<String>, raw: &str) {
    for value in raw.split(',') {
        let value = value.trim();
        if !value.is_empty() {
            values.push(value.to_string());
        }
    }
}

/// Snakemake prints `key=value, key2=value2`; only the values are kept.
fn wildcard_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter_map(|pair| {
            let pair = pair.trim();
            if pair.is_empty() {
                return None;
            }
            Some(
                pair.split_once('=')
                    .map(|(_, value)| value.trim().to_string())
                    .unwrap_or_else(|| pair.to_string()),
            )
        })
        .collect()
}

fn absolutize(outdir: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        outdir.join(path)
    }
}

fn strip_ansi(line: &str) -> String {
    lazy_regex::regex!(r"\x1b\[[0-9;?]*[ -/]*[@-~]")
        .replace_all(line, "")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::Row;

    async fn test_report(dir: &Path) -> RunReport {
        let options = SqliteConnectOptions::new()
            .filename(dir.join("report.db"))
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .unwrap();
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS snakemake (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                slurm_job_name TEXT,
                slurm_job_id TEXT,
                snakemake_job_id INTEGER NOT NULL,
                log_paths TEXT NOT NULL DEFAULT '',
                wildcards TEXT NOT NULL DEFAULT '',
                input_paths TEXT NOT NULL DEFAULT '',
                input_sizes TEXT NOT NULL DEFAULT ''
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        RunReport {
            inner: Arc::new(RunReportInner {
                pool,
                outdir: dir.to_path_buf(),
                state: Mutex::new(ReportState::default()),
            }),
        }
    }

    fn temp_dir() -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("smaug-run-report-{}-{unique}", std::process::id()))
    }

    #[tokio::test]
    async fn observes_indented_blocks_and_flushes_pending_job() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("in.txt"), b"abc").unwrap();
        let report = test_report(&dir).await;

        report.observe("localrule process:").await;
        report.observe("    input: in.txt").await;
        report.observe("    log: job.log").await;
        report.observe("    jobid: 7").await;
        report.observe("    wildcards: sample=A").await;
        report.finish().await;

        let row = sqlx::query(
            "SELECT snakemake_job_id, wildcards, input_paths, input_sizes, log_paths FROM snakemake",
        )
        .fetch_one(&report.inner.pool)
        .await
        .unwrap();

        assert_eq!(row.get::<i64, _>("snakemake_job_id"), 7);
        assert_eq!(row.get::<String, _>("wildcards"), "A");
        assert_eq!(
            row.get::<String, _>("input_paths"),
            dir.join("in.txt").display().to_string()
        );
        assert_eq!(row.get::<String, _>("input_sizes"), "3");
        assert_eq!(
            row.get::<String, _>("log_paths"),
            dir.join("job.log").display().to_string()
        );

        drop(report);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn ignores_error_rule_properties() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("in.txt"), b"abc").unwrap();
        let report = test_report(&dir).await;

        report.observe("localrule process:").await;
        report.observe("    input: in.txt").await;
        report.observe("    jobid: 7").await;
        report.observe("Error in rule process:").await;
        report.observe("    jobid: 999").await;
        report.observe("    input: other.txt").await;
        report.finish().await;

        let rows = sqlx::query("SELECT snakemake_job_id, input_paths FROM snakemake")
            .fetch_all(&report.inner.pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get::<i64, _>("snakemake_job_id"), 7);
        assert_eq!(
            rows[0].get::<String, _>("input_paths"),
            dir.join("in.txt").display().to_string()
        );

        drop(report);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_slurm_markers() {
        assert_eq!(
            parse_slurm_run_id("SLURM run ID: 8cb8c359-3930-40ec-a5df-3da736d2b4e9"),
            Some("8cb8c359-3930-40ec-a5df-3da736d2b4e9".to_string())
        );

        let submission = parse_slurm_submission(
            "Job 8 has been submitted with SLURM jobid 321 (log: /tmp/rule_a/x/321.log).",
        )
        .unwrap();
        assert_eq!(submission.snakemake_job_id, 8);
        assert_eq!(submission.slurm_job_id, "321");
        assert_eq!(
            submission.log_path.as_deref(),
            Some("/tmp/rule_a/x/321.log")
        );
    }

    #[test]
    fn parses_rule_blocks() {
        assert!(parse_rule_header("rule process:"));
        assert!(parse_rule_header("localrule init:"));
        assert!(!parse_rule_header("Finished jobid: 3 (Rule: init)"));
        assert!(is_error_rule_header("Error in rule process:"));
        assert!(!is_error_rule_header("Error in rule process"));

        assert_eq!(
            parse_block_property("    input: a.txt, b.txt"),
            Some(("input", "a.txt, b.txt"))
        );
        assert_eq!(parse_block_property("jobid: 3"), None);
    }

    #[test]
    fn keeps_only_wildcard_values() {
        assert_eq!(
            wildcard_values("sample=job_001, lane=L1"),
            vec!["job_001".to_string(), "L1".to_string()]
        );
    }
}
