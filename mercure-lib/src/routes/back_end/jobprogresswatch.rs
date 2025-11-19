use crate::pipeline_exec::watch_log;
use crate::routes::ApiResponse;
use rocket::get;

use rocket::serde::json::Json;
use serde_json::{Value, to_value};

use std::path::PathBuf;

#[get("/watch/<job_id>/<attempt_number>")]
pub async fn watch(job_id: i64, attempt_number: i64) -> Json<ApiResponse<Value>> {
    let config = crate::config::get_mercure_config();

    let logs_folder = PathBuf::from(&config.logs_dir);

    match watch_log(job_id, attempt_number, logs_folder).await {
        Ok(vars) => match to_value(&vars) {
            Ok(json_vars) => Json(ApiResponse::success(json_vars)),
            Err(e) => Json(ApiResponse::error(&format!(
                "Erreur de sérialisation: {}",
                e
            ))),
        },
        Err(e) => Json(ApiResponse::error(&format!("Erreur: {}", e))),
    }
}
