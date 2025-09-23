use rocket::State;
use rocket::http::{Cookie, CookieJar};
use rocket::serde::json::Json;
use rocket::{get, post};
use sqlx::SqlitePool;

use super::ApiResponse;
use crate::auth::AuthError;
use crate::models::{NewUser, User};

#[derive(Debug, serde::Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[post("/login", data = "<login>")]
pub async fn login(
    login: Json<LoginRequest>,
    cookies: &CookieJar<'_>,
    pool: &State<SqlitePool>,
) -> Result<Json<ApiResponse<User>>, AuthError> {
    let user = User::find_by_username(&login.username, pool)
        .await?
        .ok_or(AuthError::InvalidCredentials)?;

    if !user.verify_password(&login.password).await {
        return Err(AuthError::InvalidCredentials);
    }

    let mut user = user;
    user.update_last_login(pool).await?;

    // Créer un cookie privé (chiffré)
    cookies.add_private(Cookie::new("user_id", user.id.to_string()));

    Ok(Json(ApiResponse::success(user)))
}

#[post("/register", data = "<user>")]
pub async fn register(
    user: Json<NewUser>,
    pool: &State<SqlitePool>,
) -> Result<Json<ApiResponse<User>>, AuthError> {
    // Vérifier si l'utilisateur existe déjà
    if let Some(_) = User::find_by_username(&user.username, pool).await? {
        return Ok(Json(ApiResponse::error("Username already exists")));
    }

    let user = User::create(user.into_inner(), pool).await?;
    Ok(Json(ApiResponse::success(user)))
}

#[post("/logout")]
pub fn logout(cookies: &CookieJar<'_>) -> Json<ApiResponse<()>> {
    cookies.remove_private(Cookie::build("user_id"));
    Json(ApiResponse::success(()))
}

#[get("/welcome")]
pub fn welcome() -> &'static str {
    "Bienvenue sur Mercure! Veuillez vous connecter."
}
