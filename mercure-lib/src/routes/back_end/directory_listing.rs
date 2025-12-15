use rocket::serde::json::Json;
use rocket::{State, get};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::Authenticated;
use crate::models::directory_utils::list_directory;

/// Route: /mercure/api/directories/analysis
/// Liste les dossiers d'analyse disponibles
#[get("/directories/list?<dirtype>")]
pub async fn list_directories_by_type(
    _auth: Authenticated,
    _pool: &State<SqlitePool>,
    dirtype: Option<&str>,
) -> Json<ApiResponse<Value>> {
    let config = crate::config::get_mercure_config();
    let base_path = match dirtype {
        Some("analysis") => &config.analysis_dir,
        Some("ont") => &config.ont_dir,
        _ => {
            return Json(ApiResponse::error(
                "Type de dossier invalide. Utilisez 'analysis' ou 'ont'.".to_string(),
            ));
        }
    };
    match list_directory(base_path) {
        Ok(directories) => Json(ApiResponse::success(json!(directories))),
        Err(e) => Json(ApiResponse::error(format!(
            "Erreur lors de la lecture des dossiers d'analyse: {}",
            e
        ))),
    }
}
