use rocket::{State, get, serde::json::Json};
use serde_json::json;
use sqlx::SqlitePool;

use crate::auth::Authenticated;
use crate::models::{CachedForm, Form};
use crate::routes::ApiResponse;

/// Lists every cached FormDef straight from the database (no HTTP request to the git
/// repository). The cache is rebuilt by `POST /mercure/api/update/cache`
/// (see `forms_cache.rs`).
#[get("/forms/all")]
pub async fn get_all_forms(pool: &State<SqlitePool>) -> Json<ApiResponse<Vec<CachedForm>>> {
    match CachedForm::get_all(pool).await {
        Ok(alldefs) => Json(ApiResponse::success(alldefs)),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}

/// Lists cached FormDefs (with their `form_def_id`, `branch`, `file_path`) visible
/// to the specified group. If no group is specified, returns all forms with no groups associated
#[get("/groups/forms/<group_id>")]
pub async fn get_all_forms_for_group(
    pool: &State<SqlitePool>,
    group_id: Option<i64>,
    auth: Authenticated,
) -> Json<ApiResponse<serde_json::Value>> {
    match CachedForm::get_form_defs_for_group(pool, group_id).await {
        Ok(forms) => Json(ApiResponse::success(match group_id {
            Some(_) => {
                json!({
                    "with_group": forms,
                })
            }
            None => {
                if auth.user.is_admin {
                    json!({
                        "without_group": forms,
                    })
                } else {
                    json!({"without_groups": []})
                }
            }
        })),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}

/// Gets the full, exact form definition of a (branch, form_path) pair, directly
/// from the git repository (HTTP request).
#[get("/forms/get?<branch>&<form_path>")]
pub async fn get_form_from_id(branch: &str, form_path: &str) -> Json<ApiResponse<Form>> {
    match Form::get_form(branch, form_path).await {
        Ok(formdef) => Json(ApiResponse::success(formdef)),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}
