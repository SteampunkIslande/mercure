use rocket::serde::json::Json;
use rocket::{State, get};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::Authenticated;
use crate::models::DirectoryUtils;

/// Route: /mercure/api/directories/analysis
/// Liste les dossiers d'analyse disponibles
#[get("/directories/analysis")]
pub async fn list_analysis_directories(
    _auth: Authenticated,
    _pool: &State<SqlitePool>,
) -> Json<ApiResponse<Value>> {
    match DirectoryUtils::list_analysis_directories(None) {
        Ok(directories) => Json(ApiResponse::success(json!(directories))),
        Err(e) => Json(ApiResponse::error(format!(
            "Erreur lors de la lecture des dossiers d'analyse: {}",
            e
        ))),
    }
}

/// Route: /mercure/api/directories/analysis/custom?<base_path>
/// Liste les dossiers d'analyse dans un chemin personnalisé
#[get("/directories/analysis/custom?<base_path>")]
pub async fn list_analysis_directories_custom(
    _auth: Authenticated,
    _pool: &State<SqlitePool>,
    base_path: Option<String>,
) -> Json<ApiResponse<Value>> {
    let path = base_path.as_deref();
    match DirectoryUtils::list_analysis_directories(path) {
        Ok(directories) => Json(ApiResponse::success(json!(directories))),
        Err(e) => Json(ApiResponse::error(format!(
            "Erreur lors de la lecture des dossiers d'analyse: {}",
            e
        ))),
    }
}

/// Route: /mercure/api/directories/ont
/// Liste les dossiers ONT disponibles
#[get("/directories/ont")]
pub async fn list_ont_directories(
    _auth: Authenticated,
    _pool: &State<SqlitePool>,
) -> Json<ApiResponse<Value>> {
    match DirectoryUtils::list_ont_directories() {
        Ok(directories) => Json(ApiResponse::success(json!(directories))),
        Err(e) => Json(ApiResponse::error(format!(
            "Erreur lors de la lecture des dossiers ONT: {}",
            e
        ))),
    }
}

/// Route: /mercure/api/directories/validate?<path>
/// Valide qu'un chemin de dossier existe et est accessible
#[get("/directories/validate?<path>")]
pub async fn validate_directory_path(
    _auth: Authenticated,
    _pool: &State<SqlitePool>,
    path: String,
) -> Json<ApiResponse<Value>> {
    match DirectoryUtils::validate_directory_path(&path) {
        Ok(is_valid) => Json(ApiResponse::success(
            json!({"valid": is_valid, "path": path}),
        )),
        Err(e) => Json(ApiResponse::error(format!(
            "Erreur lors de la validation du chemin: {}",
            e
        ))),
    }
}
