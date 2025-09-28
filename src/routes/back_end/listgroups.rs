use rocket::{State, get, serde::json::Json};
use sqlx::SqlitePool;

use crate::{models::Group, routes::ApiResponse};

#[get("/groups/list")]
pub async fn list_groups(pool: &State<SqlitePool>) -> Json<ApiResponse<Vec<Group>>> {
    match Group::get_groups_with_ids(pool).await {
        Ok(groups) => Json(ApiResponse::success(groups)),
        Err(e) => Json(ApiResponse::error(format!("{:?}", e))),
    }
}
