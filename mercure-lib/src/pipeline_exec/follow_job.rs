// Ligne à extraire:
// 4 of 52 steps (8%) done
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;

pub fn follow_job(logfile: &Path) -> Result<(), String> {
    let file = File::open(logfile)
        .map_err(|e| format!("Erreur lors de l'ouverture du fichier de log: {}", e))?;
    let mut reader = BufReader::new(file);

    let mut last_job_name: Option<String> = None;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let n = reader
            .read_until(b'\n', &mut buf)
            .map_err(|e| format!("Erreur lors de la lecture du fichier de log: {}", e))?;
        if n == 0 {
            break; // fin du fichier
        }
        // On ne traite que les lignes terminées par '\n'
        if buf.last() == Some(&b'\n') {
            let line = String::from_utf8_lossy(&buf);
            if let Some(start) = line.find("SLURM run ID: ") {
                let job_name = line[start + 14..].trim();
                last_job_name = Some(job_name.to_string());
            }
        }
        // Si la ligne n'est pas terminée, on l'ignore (probablement en cours d'écriture)
    }

    let job_name = match last_job_name {
        Some(name) => name,
        None => return Err("Aucun job SLURM trouvé dans le fichier de log".to_string()),
    };

    // Annuler le job avec scancel
    let mut command = Command::new("scancel");
    command.arg(&job_name);

    eprintln!("Annulation du job SLURM avec nom {}", job_name);
    eprintln!("Commande exécutée: {:?}", command);
    let status = command
        .status()
        .map_err(|e| format!("Erreur lors de l'exécution de scancel: {}", e))?;

    if !status.success() {
        return Err(format!(
            "Échec de l'annulation du job SLURM avec nom {}: code de sortie {}",
            job_name,
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}
