use rocket::serde::json::Json;
use rocket::{State, post};
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::{AuthError, Authenticated};
use crate::models::{NewUser, User};

/// API endpoint for admins to register new users
///
/// ROUTE: /mercure/api/register
#[post("/register", data = "<user>")]
pub async fn register_post(
    user: Json<NewUser>,
    pool: &State<SqlitePool>,
    auth: Authenticated,
) -> Result<Json<ApiResponse<User>>, AuthError> {
    // Ensure user is admin
    if !auth.user.is_admin {
        return Ok(Json(ApiResponse::error("Accès non autorisé")));
    }

    // Make sure the user doesn't already exist
    if (User::find_by_usermail(&user.usermail, pool).await?).is_some() {
        return Ok(Json(ApiResponse::error("Cet utilisateur existe déjà")));
    }

    let user = User::create(user.into_inner(), pool).await?;
    Ok(Json(ApiResponse::success(user)))
}

// Group::get_user_groups(pool, auth.user.id)
