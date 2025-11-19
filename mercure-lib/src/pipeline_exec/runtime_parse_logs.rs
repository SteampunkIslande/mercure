use anyhow::Result;
use regex::Regex;
use rocket::tokio::process::Command;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Default)]
pub struct RTJobInfo {
    pub current_step_string: Option<String>,
    pub current_progress: Option<i64>,
    pub pending_jobs_count: Option<i64>,
    pub running_jobs_count: Option<i64>,
    pub done_jobs_count: Option<i64>,
}

/// Fonction asynchrone qui renvoie un flux de JobInfo en temps réel
///
/// Cette fonction surveille un fichier de log spécifique pour un job et un numéro de tentative donnés.
///
/// Pour que la lecture en temps réel fonctionne correctement,
/// le fichier de log doit être écrit ligne par ligne avec un flush après chaque écriture (`stdbuf -oL snakemake`).
///
/// Les informations renvoyées comprennent:
/// - current_step_string: la dernière étape en cours. C'est le script launcher qui doit écrire une ligne "## STEP <nom_de_l_etape>" dans stdout.
/// - current_progress: le pourcentage de progression dans l'étape actuelle. Issu de snakemake, format "X of Y steps (Z%) done".
/// - pending_jobs_count, running_jobs_count, done_jobs_count: le nombre de jobs SLURM en attente, en cours d'exécution et terminés respectivement.
pub async fn watch_log(
    job_id: i64,
    attempt_number: i64,
    logs_folder: PathBuf,
) -> Result<RTJobInfo> {
    // Recherche du fichier
    let log_pattern = format!(r"^\d+-job-{}-{}\.log$", job_id, attempt_number);
    let re_logfile = Regex::new(&log_pattern).unwrap();
    let mut entries = std::fs::read_dir(&logs_folder)?;
    let mut log_path = None;
    while let Some(entry) = entries.next() {
        let path = entry?.path();
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if re_logfile.is_match(name) {
                log_path = Some(path);
                break;
            }
        }
    }

    let log_path = log_path.ok_or_else(|| anyhow::anyhow!("Aucun fichier log trouvé"))?;
    let file = File::open(&log_path)?;
    let reader = BufReader::new(file).lines();

    let re_step = Regex::new(r"^##\s+STEP\s+(.+)$").unwrap();
    let re_slurm_id = Regex::new(r"^SLURM\s+run\s+ID:\s+(.+)$").unwrap();
    let re_progress = Regex::new(r"(\d+)\s+of\s+(\d+)\s+steps\s+\((1?\d{1,2})%\)\s+done").unwrap();

    let mut last_step: Option<String> = None;
    let mut last_slurm_id: Option<String> = None;
    let mut last_progress: Option<i64> = None;

    for line in reader {
        let line = line?;
        if let Some(cap) = re_step.captures(&line) {
            last_step = Some(cap[1].trim().to_string());
            continue;
        }
        if let Some(cap) = re_slurm_id.captures(&line) {
            last_slurm_id = Some(cap[1].trim().to_string());
            continue;
        }
        if let Some(cap) = re_progress.captures(&line) {
            let percent: i64 = cap[3].parse().unwrap_or(0);
            let part: i64 = cap[1].parse().unwrap_or(0);
            let total: i64 = cap[2].parse().unwrap_or(0);
            last_progress = Some(percent);
            if part == total {
                // pipeline terminé: réinitialiser
                last_step = None;
                last_progress = None;
                last_slurm_id = None;
            }
        }
    }

    // Return SLURM statuses for current snakemake jobs (if any)
    let (pending, running, done) = if let Some(ref slurm_id) = last_slurm_id {
        match count_jobs_in_squeue(slurm_id).await {
            Ok((p, r, d)) => (Some(p), Some(r), Some(d)),
            _ => (None, None, None),
        }
    } else {
        (None, None, None)
    };
    return Ok(RTJobInfo {
        current_step_string: last_step.clone(),
        current_progress: last_progress,
        pending_jobs_count: pending,
        running_jobs_count: running,
        done_jobs_count: done,
    });
}

async fn count_jobs_in_squeue(slurm_id: &str) -> Result<(i64, i64, i64)> {
    let output = Command::new("squeue")
        .arg("--name")
        .arg(slurm_id)
        .arg("--noheader")
        .arg("--format=%T")
        .output()
        .await?;

    if !output.status.success() {
        anyhow::bail!("squeue a échoué");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut pending = 0;
    let mut running = 0;
    let mut done = 0;

    for line in stdout.lines() {
        match line.trim() {
            "PENDING" => pending += 1,
            "RUNNING" => running += 1,
            "COMPLETED" => done += 1,
            _ => {}
        }
    }

    Ok((pending, running, done))
}
