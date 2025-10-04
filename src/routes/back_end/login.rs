use rocket::State;
use rocket::http::{Cookie, CookieJar};
use rocket::post;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use sqlx::SqlitePool;

use crate::auth::AuthError;
use crate::models::User;
use rocket::uri;

#[derive(Debug, serde::Deserialize)]
pub struct LoginRequest {
    usermail: String,
    password: String,
}

#[post("/login", data = "<login>")]
pub async fn login_post(
    login: Json<LoginRequest>,
    cookies: &CookieJar<'_>,
    pool: &State<SqlitePool>,
) -> Result<Redirect, AuthError> {
    let mut user = User::find_by_usermail(&login.usermail, pool)
        .await?
        .ok_or(AuthError::UnknownUser)?;

    if !user.verify_password(&login.password).await {
        return Err(AuthError::InvalidCredentials);
    }

    user.update_last_login(pool).await?;

    let mut cookie = Cookie::new("user_id", user.id.to_string());

    // Crée un cookie pour la durée de la session (dépend du navigateur)
    cookie.set_expires(None);
    cookies.add_private(cookie);

    if user.is_admin {
        Ok(Redirect::to(uri!("/mercure/admin/dashboard")))
    } else {
        Ok(Redirect::to(uri!("/mercure/home")))
    }
}
