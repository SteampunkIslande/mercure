/// Ce module contient des fonctions pour vérifier les révisions git des fichiers de lanceur.
use crate::config;
use crate::models::HgFormDef;
use crate::models::ModelError;
use sqlx::Row;
use sqlx::SqlitePool;
use std::path::Path;
use std::process::Command;

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
pub fn get_current_revision(launcher_path: &Path) -> Option<String> {
    let config = config::get_mercure_config();

    let pipelines_dir = Path::new(&config.pipeline_dir);

    // Ensure the path is tracked by git
    let file_status_is_clean = Command::new("git")
        .current_dir(pipelines_dir)
        .arg("status")
        .arg("--porcelain")
        .arg("--")
        .arg(launcher_path.strip_prefix(&pipelines_dir).ok()?)
        .output()
        .ok()?
        .stdout
        .is_empty();
    if !file_status_is_clean {
        return None;
    } else {
        String::from_utf8(
            Command::new("git")
                .current_dir(pipelines_dir)
                .arg("rev-list")
                .arg("-n")
                .arg("1")
                .arg("HEAD")
                .arg("--")
                .arg(launcher_path.strip_prefix(&pipelines_dir).ok()?)
                .output()
                .ok()?
                .stdout,
        )
        .ok()
    }
}

/// Public helper: get the current git revision for the launcher referenced by a form definition.
/// Returns None if the file is dirty/untracked or any git error occurs.
pub fn get_current_launcher_revision_for_form(form: &HgFormDef) -> Option<String> {
    let config = config::get_mercure_config();
    let launcher_path = Path::new(&config.pipeline_dir)
        .join(&form.pipeline_name)
        .join("launchers")
        .join(&form.launcher_name);
    get_current_revision(&launcher_path)
}

/// Trouve une ou plusieurs définitions de formulaire qui correspondent au nom de pipeline + formulaire donné et pointent vers la révision de lanceur donnée.
///
/// Renvoie Ok(Vec<(form_id, form_name, version)>) si trouvé, ou Err en cas d'erreur de base de données.
/// Arguments:
/// - pool: Référence au pool de connexions SQLite
/// - pipeline_name: Nom du pipeline
/// - form_name: Nom du formulaire
/// - revision: Révision du lanceur
///
/// Returns:
/// - Result<Vec<(i64, String, i64)>, ModelError>: Vecteur de tuples contenant (form_id, form_name, version) ou une erreur de modèle
pub async fn find_forms_with_revision(
    pool: &SqlitePool,
    pipeline_name: &str,
    form_name: &str,
    revision: &str,
) -> Result<Vec<(i64, String, i64)>, ModelError> {
    let rows = sqlx::query(
        r#"
        SELECT form_id,form_name,version FROM Formdef WHERE pipeline_name = ? AND form_name = ? AND latest_launcher_revision = ? ORDER BY version DESC
        "#,
    )
    .bind(pipeline_name)
    .bind(form_name)
    .bind(revision)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            Ok((
                r.try_get::<i64, &str>("form_id")?,
                r.try_get::<String, &str>("form_name")?,
                r.try_get::<i64, &str>("version")?,
            ))
        })
        .collect::<Result<Vec<(i64, String, i64)>, sqlx::Error>>()?)
}

pub async fn check_launcher_exists(pipeline_name: &str, launcher_name: &str) -> bool {
    let config = config::get_mercure_config();
    let launcher_path = Path::new(&config.pipeline_dir)
        .join(pipeline_name)
        .join("launchers")
        .join(launcher_name);
    launcher_path.exists()
}
