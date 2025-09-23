use rocket::futures::stream::Forward;
use rocket::http::CookieJar;
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::Redirect;
use sqlx::SqlitePool;

use super::AuthError;
use crate::models::User;

pub struct Authenticated {
    pub user: User,
}

async fn user_from_cookie<'r>(request: &'r Request<'_>) -> Result<User, AuthError> {
    let cookies = request.cookies();

    let pool = match request.rocket().state::<SqlitePool>() {
        None => {
            return Err(AuthError::DatabaseError(
                "Could not find managed sqlite pool".to_string(),
            ));
        }
        Some(pool) => pool,
    };

    match cookies.get_private("user_id") {
        Some(cookie) => {
            let user_id: i64 = cookie
                .value()
                .parse()
                .map_err(|err| AuthError::TokenError)?;
            User::find_by_id(user_id, pool)
                .await?
                .ok_or(AuthError::DatabaseError(format!(
                    "Cannot find user {}",
                    user_id
                )))
        }
        None => Err(AuthError::TokenError),
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Vérifie si l'utilisateur est authentifié via un cookie de session
        match user_from_cookie(request).await {
            Ok(user) => Outcome::Success(Authenticated { user }),
            Err(_) => Outcome::Error((Status::Unauthorized, AuthError::TokenError)),
        }
    }
}

pub fn is_authenticated(cookies: &CookieJar<'_>) -> bool {
    cookies.get_private("user_id").is_some()
}
