use rocket::{State, get, serde::json::Json};
use sqlx::SqlitePool;

use crate::{auth::AuthError, models::Group, routes::ApiResponse};

#[get("/groups/list")]
pub async fn list_groups(
    pool: &State<SqlitePool>,
) -> Result<Json<ApiResponse<Vec<Group>>>, AuthError> {
    let groups = Group::get_groups_with_ids(pool).await?;
    Ok(Json(ApiResponse::success(groups)))
}

#[get("/groups/list/<user_id>")]
pub async fn list_groups_for_user(
    pool: &State<SqlitePool>,
    user_id: i64,
) -> Result<Json<ApiResponse<Vec<Group>>>, AuthError> {
    let groups = Group::get_user_groups(pool, user_id).await?;
    Ok(Json(ApiResponse::success(groups)))
}
