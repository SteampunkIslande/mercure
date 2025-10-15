use regex::Regex;
use std::collections::HashMap;
use std::io::Read;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, PartialEq)]
pub struct JobInfo {
    pub slurm_id: String,
    pub log_path: String,
    pub rule_name: String,
    pub wildcards: String,
}

/// Cette fonction lit un fichier de log ligne par ligne, et enregistre, pour chaque workflow lancé,
/// les jobs associés avec leurs informations (ID SLURM, chemin du log, nom de la règle, wildcards).
/// Elle retourne une HashMap où la clé est un tuple (run_id, order) et la valeur est une liste de JobInfo.
pub fn parse_log_file(file: impl Read) -> Result<HashMap<(String, u32), Vec<JobInfo>>, String> {
    let reader = BufReader::new(file);

    let re_run_id = Regex::new(r"SLURM run ID:\s*([a-f0-9-]+)").unwrap();
    let re_job = Regex::new(
        r"Job\s+\d+\s+has been submitted with SLURM jobid\s+(\d+|\d{3})\s+\(log:\s+([^)]+)\)",
    )
    .unwrap();

    let mut runs: HashMap<(String, u32), Vec<JobInfo>> = HashMap::new();
    let mut current_run_id: Option<String> = None;
    let mut current_order: u32 = 0;

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        println!("Processing line: {}", line);

        // Détection d’un nouveau workflow
        if let Some(cap) = re_run_id.captures(&line) {
            let run_id = cap[1].to_string();
            current_run_id = Some(run_id.clone());
            current_order = 0;
            runs.insert((run_id, current_order), Vec::new());

            continue;
        }

        // Détection d’un job associé
        if let Some(cap) = re_job.captures(&line) {
            if current_run_id.is_some() {
                let slurm_id = cap[1].to_string();
                let log_path = cap[2].to_string();

                let rule_name = Path::new(&log_path)
                    .parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("no_wildcards")
                    .to_string();

                // Extraire le dernier dossier parent
                let wildcards = Path::new(&log_path)
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|s| s.to_str())
                    .unwrap_or("no_wildcards")
                    .to_string();

                if let Some(run_id) = current_run_id.as_ref() {
                    runs.entry((run_id.clone(), current_order)).and_modify(|v| {
                        v.push(JobInfo {
                            slurm_id,
                            log_path,
                            rule_name,
                            wildcards,
                        })
                    });
                }
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
Job 1 has been submitted with SLURM jobid 321 (log: /full/path/to/log/rule_a/wildcardA_wildcardB/321.log).
Job 2 has been submitted with SLURM jobid 322 (log: /full/path/to/log/rule_b/wildcardA_wildcardB/322.log).
SLURM run ID: 92db70f3-2c3b-4b45-9f5a-d01c2c49f821
Job 1 has been submitted with SLURM jobid 420 (log: /full/path/to/log/rule_DD/wildcardAC_wildcardDC/420.log)."#;
        let parsed = parse_log_file(test_log_string.as_bytes()).unwrap();
        assert_eq!(
            parsed.get(&(String::from("8cb8c359-3930-40ec-a5df-3da736d2b4e9"), 0)),
            Some(&vec![
                JobInfo {
                    slurm_id: "321".to_string(),
                    log_path: "/full/path/to/log/rule_a/wildcardA_wildcardB/321.log".to_string(),
                    rule_name: "rule_a".to_string(),
                    wildcards: "wildcardA_wildcardB".to_string()
                },
                JobInfo {
                    slurm_id: "322".to_string(),
                    log_path: "/full/path/to/log/rule_b/wildcardA_wildcardB/322.log".to_string(),
                    rule_name: "rule_b".to_string(),
                    wildcards: "wildcardA_wildcardB".to_string()
                }
            ])
        );
        assert_eq!(
            parsed.get(&(String::from("92db70f3-2c3b-4b45-9f5a-d01c2c49f821"), 0)),
            Some(&vec![JobInfo {
                slurm_id: "420".to_string(),
                log_path: "/full/path/to/log/rule_DD/wildcardAC_wildcardDC/420.log".to_string(),
                rule_name: "rule_DD".to_string(),
                wildcards: "wildcardAC_wildcardDC".to_string()
            },])
        );
    }
}
