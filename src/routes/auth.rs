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

/// Route to try logging in from post request
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

    // Créer un cookie privé (chiffré)
    cookies.add_private(Cookie::new("user_id", user.id.to_string()));

    if user.is_admin {
        eprintln!("User is admin");
        Ok(Redirect::to(uri!("/mercure/admin/dashboard")))
    } else {
        eprintln!("User is not admin");
        Ok(Redirect::to(uri!("/mercure/home")))
    }
}
