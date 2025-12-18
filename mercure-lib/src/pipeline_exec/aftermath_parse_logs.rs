use lazy_regex::Regex;
use std::collections::HashMap;
use std::io::Read;
use std::io::{BufRead, BufReader};
use std::path::Path;

use lazy_regex::{Lazy, lazy_regex};

pub static RE_RUN_ID: Lazy<Regex> = lazy_regex!(r#"SLURM run ID:\s+(.+)"#);
pub static RE_JOB: Lazy<Regex> = lazy_regex!(
    r#"Job\s+\d+\s+has\s+been\s+submitted\s+with\s+SLURM\s+jobid\s+(\d+)\s+\(log:\s+([^)]+)\)"#
);

#[derive(Debug, PartialEq)]
pub struct JobInfo {
    pub slurm_job_name: String,
    pub slurm_job_id: String,
    pub log_path: String,
    pub rule_name: String,
    pub wildcards: Option<String>,
}

/// Parse le nom du fichier de log pour extraire le nom de la règle et les wildcards (si présents)
///
/// Cette fonction suppose que le fichier de log suit la structure:
/// /.../slurm_logs/<rule_name>/<wildcards>/<log_file>.log
/// ou
/// /.../slurm_logs/<rule_name>/<log_file>.log
///
/// Retourne un tuple (rule_name, Option<wildcards>)
pub fn parse_log_file_name(log_path_str: &str) -> (String, Option<String>) {
    let log_path = Path::new(log_path_str);

    let parents_count_before_slurm_logs = log_path
        .ancestors()
        .take_while(|p| p.file_name() != Some("slurm_logs".as_ref()))
        .count();

    let (rule_name, wildcards) = if parents_count_before_slurm_logs == 3 {
        let rule_name = log_path
            .ancestors()
            .nth(parents_count_before_slurm_logs - 1)
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("unknown_rule")
            .to_string();

        let wildcards = log_path
            .ancestors()
            .nth(parents_count_before_slurm_logs - 2)
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("unknown_wildcards")
            .to_string();

        (rule_name, Some(wildcards))
    } else {
        // Cas sans wildcard
        let rule_name = log_path
            .ancestors()
            .nth(parents_count_before_slurm_logs - 1)
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("unknown_rule")
            .to_string();
        (rule_name, None)
    };
    (rule_name, wildcards)
}

/// Cette fonction lit un fichier de log ligne par ligne, et enregistre, pour chaque workflow lancé,
/// les jobs associés avec leurs informations (Nom du job SLURM, ID du job SLURM, chemin du log, nom de la règle, wildcards).
///
/// Elle retourne une HashMap où la clé est un tuple (run_id, order) et la valeur est une liste de JobInfo.
///
/// Cette fonction ne doit être appelée qu'à la fin de l'exécution du pipeline, pour générer les différents rapports.
///
/// Tous les rapports se trouvent dans une archive zip, stockée dans /OPT/JOBS/REPORTS
///
/// Cette archive contient:
/// - Un rapport au format PDF avec l'efficacité des jobs, où les titres H1 sont les règles, et le contenu est un tableau avec pour colonnes: `Wildcards|Mémoire utilisée|Durée d'exécution|Pourcentage mémoire utilisée/mémoire demandée à SLURM|Pourcentage durée d'exécution/durée demandée à SLURM`
/// - Les fichiers de log, triés par nom de règle et par wildcard, exactement selon la structure générée par le plugin snakemake SLURM executor
pub fn parse_log_file(file: impl Read) -> Result<HashMap<u32, Vec<JobInfo>>, String> {
    let reader = BufReader::new(file);

    let mut runs: HashMap<u32, Vec<JobInfo>> = HashMap::new();
    let mut current_run_id: Option<String> = None;
    let mut current_order: u32 = 0;

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;

        // Détection d’un nouveau workflow
        if let Some(cap) = RE_RUN_ID.captures(&line) {
            let run_id = cap[1].to_string();
            current_run_id = Some(run_id.clone());
            current_order += 1;
            runs.insert(current_order, Vec::new());

            continue;
        }

        // Détection d’un job associé
        if let Some(cap) = RE_JOB.captures(&line)
            && current_run_id.is_some()
        {
            let slurm_id = cap[1].to_string();
            let log_path_str = cap[2].to_string();

            let (rule_name, wildcards) = parse_log_file_name(&log_path_str);

            if let Some(run_id) = current_run_id.as_ref() {
                runs.entry(current_order).and_modify(|v| {
                    v.push(JobInfo {
                        slurm_job_name: run_id.clone(),
                        slurm_job_id: slurm_id,
                        log_path: log_path_str,
                        rule_name,
                        wildcards,
                    })
                });
            }
        }
    }

    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_file() {
        let test_log_string: &'static str = r#"SLURM run ID: 8cb8c359-3930-40ec-a5df-3da736d2b4e9
Job 1 has been submitted with SLURM jobid 321 (log: /full/path/to/slurm_logs/rule_a/wildcardA_wildcardB/321.log).
Job 2 has been submitted with SLURM jobid 322 (log: /full/path/to/slurm_logs/rule_b/wildcardA_wildcardB/322.log).
SLURM run ID: 92db70f3-2c3b-4b45-9f5a-d01c2c49f821
Job 1 has been submitted with SLURM jobid 420 (log: /full/path/to/slurm_logs/rule_DD/wildcardAC_wildcardDC/420.log).
Job 2 has been submitted with SLURM jobid 421 (log: /full/path/to/slurm_logs/rule_DD/421.log)."#;
        let parsed = parse_log_file(test_log_string.as_bytes()).unwrap();
        assert_eq!(
            parsed.get(&1),
            Some(&vec![
                JobInfo {
                    slurm_job_name: "8cb8c359-3930-40ec-a5df-3da736d2b4e9".to_string(),
                    slurm_job_id: "321".to_string(),
                    log_path: "/full/path/to/slurm_logs/rule_a/wildcardA_wildcardB/321.log"
                        .to_string(),
                    rule_name: "rule_a".to_string(),
                    wildcards: Some("wildcardA_wildcardB".to_string())
                },
                JobInfo {
                    slurm_job_name: "8cb8c359-3930-40ec-a5df-3da736d2b4e9".to_string(),
                    slurm_job_id: "322".to_string(),
                    log_path: "/full/path/to/slurm_logs/rule_b/wildcardA_wildcardB/322.log"
                        .to_string(),
                    rule_name: "rule_b".to_string(),
                    wildcards: Some("wildcardA_wildcardB".to_string())
                }
            ])
        );
        assert_eq!(
            parsed.get(&2),
            Some(&vec![
                JobInfo {
                    slurm_job_name: "92db70f3-2c3b-4b45-9f5a-d01c2c49f821".to_string(),
                    slurm_job_id: "420".to_string(),
                    log_path: "/full/path/to/slurm_logs/rule_DD/wildcardAC_wildcardDC/420.log"
                        .to_string(),
                    rule_name: "rule_DD".to_string(),
                    wildcards: Some("wildcardAC_wildcardDC".to_string())
                },
                JobInfo {
                    slurm_job_name: "92db70f3-2c3b-4b45-9f5a-d01c2c49f821".to_string(),
                    slurm_job_id: "421".to_string(),
                    log_path: "/full/path/to/slurm_logs/rule_DD/421.log".to_string(),
                    rule_name: "rule_DD".to_string(),
                    wildcards: None
                }
            ])
        );
    }
}
