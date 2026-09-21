use std::path::PathBuf;

use rocket::{State, get, serde::json::Json};
use serde_json::json;
use sqlx::SqlitePool;

use crate::config::MercureConfig;
use crate::models::Form;
use crate::routes::ApiResponse;

#[get("/forms/all")]
pub async fn get_all_forms(config: &State<MercureConfig>) -> Json<ApiResponse<Vec<Form>>> {
    match Form::get_all_form_defs(&config).await {
        Ok(alldefs) => Json(ApiResponse::success(alldefs)),
        Err(e) => Json(ApiResponse::error(format!("{:?}", e))),
    }
}

///<branch>/<form_path..>
#[get("/forms/groups/<group_id>")]
pub async fn get_all_forms_for_group(
    pool: &State<SqlitePool>,
    config: &State<MercureConfig>,
    group_id: i64,
) -> Json<ApiResponse<serde_json::Value>> {
    match Form::get_form_list_items_for_group(pool, config, group_id).await {
        Ok(alldefs) => Json(ApiResponse::success(json!({
            "with_group": alldefs.0,
            "without_group": alldefs.1
        }))),
        Err(e) => Json(ApiResponse::error(format!("{:?}", e))),
    }
}

#[get("/forms/<branch>/<form_path..>")]
pub async fn get_form_from_id(branch: String, form_path: PathBuf) -> Json<ApiResponse<Form>> {
    match Form::get_form(&branch, &form_path).await {
        Ok(formdef) => Json(ApiResponse::success(formdef)),
        Err(e) => Json(ApiResponse::error(format!("{:?}", e))),
    }
}
