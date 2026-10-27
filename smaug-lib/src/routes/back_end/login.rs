use anyhow::Context;
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
        .context(AuthError::UnknownUser)?;

    if !user.verify_password(&login.password).await {
        return Err(AuthError::InvalidCredentials);
    }

    user.update_last_login(pool).await?;

    let mut cookie = Cookie::new("user_id", user.id.to_string());

    // Create a cookie for the duration of the session (depends on the browser)
    cookie.set_expires(None);
    cookies.add_private(cookie);

    if user.is_admin {
        Ok(Redirect::to(uri!("/smaug/admin/dashboard")))
    } else {
        Ok(Redirect::to(uri!("/smaug/home")))
    }
}
