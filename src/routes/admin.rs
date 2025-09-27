use rocket::serde::json::Json;
use rocket::{State, get, post};
use sqlx::SqlitePool;

use super::ApiResponse;
use crate::auth::{AuthError, Authenticated};
use crate::models::{NewUser, User};

use rocket_dyn_templates::{Template, context};

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
    if let Some(_) = User::find_by_usermail(&user.usermail, pool).await? {
        return Ok(Json(ApiResponse::error("Cet utilisateur existe déjà")));
    }

    let user = User::create(user.into_inner(), pool).await?;
    Ok(Json(ApiResponse::success(user)))
}

/// Route to show the admin a page to register new user.
#[get("/register")]
pub async fn register_get(auth: Authenticated) -> Option<rocket::fs::NamedFile> {
    if !auth.user.is_admin {
        return None;
    }
    rocket::fs::NamedFile::open("static/admin/register.html")
        .await
        .ok()
}

/// Admin landing page
#[get("/landing_page")]
pub async fn admin_landing_page_get(auth: Authenticated) -> Option<Template> {
    if !auth.user.is_admin {
        return None;
    }
    Some(Template::render(
        "admin/landing_page",
        context! {user:auth.user},
    ))
}
