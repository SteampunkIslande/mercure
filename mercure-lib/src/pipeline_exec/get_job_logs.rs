use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parcourt le fichier de log SLURM et extrait pour chaque job :
/// - Le job ID SLURM (String)
/// - Son rang d'apparition (u64)
/// - Les wildcards (Vec<String>, extraites du chemin du log)
/// - Le chemin absolu du log (String)
///
/// Retourne une HashMap dont la clé est (job_id_slurm, rang) et la valeur est (wildcards, chemin_log).
pub fn get_job_logs(
    logfile: &Path,
) -> Result<HashMap<(String, u64), (Vec<String>, String)>, String> {
    let file = File::open(logfile)
        .map_err(|e| format!("Erreur lors de l'ouverture du fichier de log: {}", e))?;
    let reader = BufReader::new(file);

    let mut result: HashMap<(String, u64), (Vec<String>, String)> = HashMap::new();
    let mut rang: u64 = 0;

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Erreur de lecture: {}", e))?;
        // Cherche la ligne de soumission de job
        if let Some(start) = line.find("Job ") {
            if let Some(snakemake_id_end) =
                line[start + 4..].find(" has been submitted with SLURM jobid ")
            {
                let snakemake_id = &line[start + 4..start + 4 + snakemake_id_end];
                let rest = &line[start + 4 + snakemake_id_end + 32..]; // 32 = len(" has been submitted with SLURM jobid ")
                if let Some(slurm_id_end) = rest.find(" (log: ") {
                    let slurm_id = &rest[..slurm_id_end];
                    let log_path_start = slurm_id_end + 7; // 7 = len(" (log: ")
                    if let Some(log_path_end) = rest[log_path_start..].find(").") {
                        let log_path = &rest[log_path_start..log_path_start + log_path_end];
                        // Extraction des wildcards depuis le chemin du log
                        let wildcards = extract_wildcards_from_log_path(log_path);
                        result.insert(
                            (slurm_id.to_string(), rang),
                            (wildcards, log_path.to_string()),
                        );
                        rang += 1;
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Exemple d'extraction des wildcards depuis le chemin du log.
/// À adapter selon le format réel des chemins de log.
fn extract_wildcards_from_log_path(log_path: &str) -> Vec<String> {
    // Supposons que les wildcards sont des segments entre '/' dans le chemin
    // et qu'ils sont placés après un dossier 'logs/'.
    // Exemple: /chemin/vers/logs/wildcard1/wildcard2/slurm-job.log
    let parts: Vec<&str> = log_path.split('/').collect();
    if let Some(pos) = parts.iter().position(|&x| x == "logs") {
        // On prend les segments après 'logs' jusqu'à l'avant-dernier (avant le nom du fichier)
        if parts.len() > pos + 2 {
            return parts[pos + 1..parts.len() - 1]
                .iter()
                .map(|s| s.to_string())
                .collect();
        }
    }
    Vec::new()
}
