use rocket::serde::json::Json;
use rocket::{State, get, post};
use sqlx::SqlitePool;

use super::ApiResponse;
use crate::auth::{AuthError, Authenticated};
use crate::models::{NewUser, User};

#[post("/register", data = "<user>")]
pub async fn register(
    user: Json<NewUser>,
    pool: &State<SqlitePool>,
    auth: Authenticated,
) -> Result<Json<ApiResponse<User>>, AuthError> {
    // Vérifier si l'utilisateur est admin
    if !auth.user.is_admin {
        return Ok(Json(ApiResponse::error("Accès non autorisé")));
    }

    // Vérifier si l'utilisateur existe déjà
    if let Some(_) = User::find_by_username(&user.username, pool).await? {
        return Ok(Json(ApiResponse::error("Ce nom d'utilisateur existe déjà")));
    }

    let user = User::create(user.into_inner(), pool).await?;
    Ok(Json(ApiResponse::success(user)))
}

#[get("/register")]
pub async fn register_page(auth: Authenticated) -> Option<rocket::fs::NamedFile> {
    if !auth.user.is_admin {
        return None;
    }
    rocket::fs::NamedFile::open("templates/admin/register.html")
        .await
        .ok()
}
