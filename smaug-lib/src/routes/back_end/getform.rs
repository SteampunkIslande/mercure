use rocket::{State, get, serde::json::Json};
use serde_json::json;
use sqlx::SqlitePool;

use crate::auth::{AuthError, Authenticated};
use crate::models::CachedForm;
use crate::routes::ApiResponse;

/// Lists every cached FormDef straight from the database (no HTTP request to the git
/// repository). The cache is rebuilt by `POST /smaug/api/update/cache`
/// (see `forms_cache.rs`).
#[get("/forms/all")]
pub async fn get_all_forms(
    pool: &State<SqlitePool>,
) -> Result<Json<ApiResponse<Vec<CachedForm>>>, AuthError> {
    let alldefs = CachedForm::get_all(pool).await?;
    Ok(Json(ApiResponse::success(alldefs)))
}

/// Lists cached FormDefs (with their `form_def_id`, `branch`, `file_path`) visible
/// to the specified group. If no group is specified, returns all forms with no groups associated
#[get("/groups/forms/<group_id>")]
pub async fn get_all_forms_for_group(
    pool: &State<SqlitePool>,
    group_id: Option<i64>,
    auth: Authenticated,
) -> Result<Json<ApiResponse<serde_json::Value>>, AuthError> {
    let forms = CachedForm::get_form_defs_for_group(pool, group_id).await?;
    Ok(Json(ApiResponse::success(match group_id {
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
    })))
}
