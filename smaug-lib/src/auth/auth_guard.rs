use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use sqlx::SqlitePool;

use super::AuthError;
use crate::models::User;

pub struct Authenticated {
    pub user: User,
}

async fn user_from_cookie(request: &Request<'_>) -> Result<User, AuthError> {
    let cookies = request.cookies();

    let pool = request
        .rocket()
        .state::<SqlitePool>()
        .ok_or_else(|| anyhow::anyhow!("Could not find managed sqlite pool"))?;

    let cookie = cookies
        .get_private("user_id")
        .ok_or_else(|| anyhow::Error::msg("Could not find cookie with key `user_id`"))?;

    let user_id: i64 = cookie
        .value()
        .parse()
        .map_err(|err| anyhow::Error::msg(format!("Could not parse int from cookie: {err}")))?;

    Ok(User::find_by_id(user_id, pool).await?)
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Vérifie si l'utilisateur est authentifié via un cookie de session
        match user_from_cookie(request).await {
            Ok(user) => Outcome::Success(Authenticated { user }),
            Err(err) => Outcome::Error((Status::Unauthorized, err)),
        }
    }
}
