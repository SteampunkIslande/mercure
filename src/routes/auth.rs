use rocket::State;
use rocket::http::{Cookie, CookieJar};
use rocket::post;
use rocket::serde::json::Json;
use sqlx::SqlitePool;

use super::ApiResponse;
use crate::auth::AuthError;
use crate::models::User;

#[derive(Debug, serde::Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[post("/login", data = "<login>")]
pub async fn login_post(
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
