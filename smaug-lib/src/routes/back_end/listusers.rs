use rocket::{State, get, serde::json::Json};
use sqlx::SqlitePool;

use crate::{auth::AuthError, models::User, routes::ApiResponse};

#[get("/users/list")]
pub async fn list_users(
    pool: &State<SqlitePool>,
) -> Result<Json<ApiResponse<Vec<User>>>, AuthError> {
    let users = User::list_users(pool).await?;
    Ok(Json(ApiResponse::success(users)))
}
