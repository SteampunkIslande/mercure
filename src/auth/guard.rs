use rocket::http::CookieJar;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::Redirect;

use super::AuthError;
use crate::models::User;

pub struct Authenticated {
    pub user: User,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authenticated {
    type Error = AuthError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Vérifie si l'utilisateur est authentifié via un cookie de session
        let cookies = request.cookies();

        match cookies.get_private("user_id") {
            Some(cookie) => {
                // TODO: Récupérer l'utilisateur depuis la base de données
                // Pour l'instant, on retourne une erreur
                Outcome::Error((Status::Unauthorized, AuthError::TokenError))
            }
            None => Outcome::Error((Status::Unauthorized, AuthError::InvalidCredentials)),
        }
    }
}

pub fn is_authenticated(cookies: &CookieJar<'_>) -> bool {
    cookies.get_private("user_id").is_some()
}
