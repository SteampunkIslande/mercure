use rocket::{State, get, serde::json::Json};
use serde_json::json;
use sqlx::SqlitePool;

use crate::{auth::Authenticated, models::Group, routes::ApiResponse};

/// API endpoint to create a new group from its name
///
/// ROUTE: /mercure/api/newgroup/<group_name>
#[get("/newgroup/<group_name>")]
pub async fn newgroup_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    group_name: &str,
) -> Json<ApiResponse<serde_json::Value>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "You are not allowed to create new group, only admins can!".to_string(),
        ));
    }
    match Group::add_group(pool, group_name).await {
        Ok(group_id) => Json(ApiResponse::success(json!( {
            "group_id":group_id,
            "group_name":group_name
        }))),
        Err(e) => Json(ApiResponse::error(format!("{:?}", e))),
    }
}
