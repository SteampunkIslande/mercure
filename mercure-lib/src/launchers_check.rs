/// Ce module contient des fonctions pour vérifier les révisions git des fichiers de lanceur.
use crate::config;
use crate::models::HgFormDef;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LauncherCheckError {
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error("Le fichier launcher n'est pas suivi par git.")]
    UntrackedFile,
    #[error("Le fichier launcher a des modifications non validées.")]
    UncommittedChanges,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    StripPrefixError(#[from] std::path::StripPrefixError),
}

/// Get the latest git revision of a launcher file
///
/// Returns None if the file is not tracked by git or if there are any errors
/// in executing git commands
///
/// Arguments:
/// - launcher_path: Path to the launcher file
///
/// Returns:
///
/// - Option<String>: Latest git revision as a String, or None if not tracked or error
pub fn get_current_revision(launcher_path: &Path) -> Result<String, LauncherCheckError> {
    let config = config::get_mercure_config();

    let pipelines_dir = Path::new(&config.pipeline_dir);

    // Ensure the path is tracked by git
    let file_status = Command::new("git")
        .current_dir(pipelines_dir)
        .arg("status")
        .arg("--porcelain")
        .arg("--")
        .arg(launcher_path.strip_prefix(pipelines_dir)?)
        .output()?
        .stdout;
    let file_status_is_clean = file_status.is_empty();
    if !file_status_is_clean {
        let (index_state, working_state) = (file_status[0] as char, file_status[1] as char);
        match (index_state, working_state) {
            ('?', _) | (_, '?') => return Err(LauncherCheckError::UntrackedFile),
            ('M', _) | (_, 'M') | ('A', _) | (_, 'A') | ('D', _) | (_, 'D') => {
                return Err(LauncherCheckError::UncommittedChanges);
            }
            _ => {}
        }
    }
    Ok(String::from_utf8(
        Command::new("git")
            .current_dir(pipelines_dir)
            .arg("rev-list")
            .arg("-n")
            .arg("1")
            .arg("HEAD")
            .arg("--")
            .arg(launcher_path.strip_prefix(pipelines_dir)?)
            .output()?
            .stdout,
    )?)
}

/// Public helper: get the current git revision for the launcher referenced by a form definition.
/// Returns None if the file is dirty/untracked or any git error occurs.
pub fn get_current_launcher_revision_for_form(
    form: &HgFormDef,
) -> Result<String, LauncherCheckError> {
    let config = config::get_mercure_config();
    let launcher_path = Path::new(&config.pipeline_dir)
        .join(&form.pipeline_name)
        .join("launchers")
        .join(&form.launcher_name);
    get_current_revision(&launcher_path)
}

pub async fn check_launcher_exists(pipeline_name: &str, launcher_name: &str) -> bool {
    let config = config::get_mercure_config();
    let launcher_path = Path::new(&config.pipeline_dir)
        .join(pipeline_name)
        .join("launchers")
        .join(launcher_name);
    launcher_path.exists()
}

/// Vérifie si un pipeline est archivé (la révision git actuelle ne correspond pas à celle stockée dans le formulaire)
///
/// Un pipeline est considéré comme archivé si:
/// - Le launcher existe toujours
/// - Mais la révision git actuelle du launcher ne correspond pas à celle enregistrée dans le formulaire
///
/// Arguments:
/// - form: Référence vers la définition de formulaire
///
/// Returns:
/// - bool: true si le pipeline est archivé, false sinon
pub fn is_pipeline_archived(form: &HgFormDef) -> bool {
    // Si nous n'avons pas de révision enregistrée, on ne peut pas savoir si c'est archivé
    let stored_revision = match &form.latest_launcher_revision {
        Some(rev) => rev,
        None => return false,
    };

    // Obtenons la révision actuelle
    match get_current_launcher_revision_for_form(form) {
        Ok(current_revision) => {
            // Comparer les révisions (en supprimant les espaces en début/fin)
            stored_revision.trim() != current_revision.trim()
        }
        Err(_) => {
            // Si on ne peut pas obtenir la révision actuelle, on considère que c'est archivé
            // car soit le fichier n'existe plus, soit il y a un problème git
            true
        }
    }
}
