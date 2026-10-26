use rocket::{State, get, serde::json::Json};
use serde_json::json;
use sqlx::SqlitePool;

use crate::{
    auth::{AuthError, Authenticated},
    models::Group,
    routes::ApiResponse,
};

/// True when the error is a violated `UNIQUE` constraint (e.g. duplicate group
/// name). Used to keep the user-facing "already exists" answer instead of
/// exposing the database error as an internal failure.
fn is_unique_violation(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<sqlx::Error>()
        .and_then(|e| match e {
            sqlx::Error::Database(db) => Some(db.kind() == sqlx::error::ErrorKind::UniqueViolation),
            _ => None,
        })
        .unwrap_or(false)
}

/// API endpoint to create a new group from its name
///
/// ROUTE: /smaug/api/newgroup/<group_name>
#[get("/newgroup/<group_name>")]
pub async fn newgroup_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    group_name: &str,
) -> Result<Json<ApiResponse<serde_json::Value>>, AuthError> {
    if !auth.user.is_admin {
        return Err(AuthError::AdminsOnly);
    }
    match Group::add_group(pool, group_name).await {
        Ok(group_id) => Ok(Json(ApiResponse::success(json!( {
            "group_id":group_id,
            "group_name":group_name
        })))),
        // Another group with this name already exists: tell the user (409) rather
        // than exposing the UNIQUE constraint violation as an internal error.
        Err(e) if is_unique_violation(&e) => {
            Err(anyhow::Error::new(AuthError::DuplicateGroup(group_name.to_string())).into())
        }
        Err(e) => Err(e
            .context(format!("Impossible d'ajouter le groupe `{group_name}`"))
            .into()),
    }
}
