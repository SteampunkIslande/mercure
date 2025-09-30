use rocket::{State, get, serde::json::Json};
use sqlx::SqlitePool;

use crate::{models::User, routes::ApiResponse};

#[get("/users/list")]
pub async fn list_users(pool: &State<SqlitePool>) -> Json<ApiResponse<Vec<User>>> {
    match User::list_users(pool).await {
        Ok(groups) => Json(ApiResponse::success(groups)),
        Err(e) => Json(ApiResponse::error(format!("{:?}", e))),
    }
}
