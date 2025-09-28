mod guard;

use std::io::Cursor;

use super::routes::ApiResponse;

pub use guard::*;

use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Unknown user")]
    UnknownUser,
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Token error: {0}")]
    TokenError(String),
}

impl<'r> Responder<'r, 'static> for AuthError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let mut response = Response::build();
        let body = match self {
            AuthError::InvalidCredentials => {
                response.status(Status::Unauthorized);
                ApiResponse::<u8>::error("Mote de passe invalide".to_string())
            }
            AuthError::DatabaseError(e) => {
                response.status(Status::InternalServerError);
                ApiResponse::<u8>::error(format!("Erreur de la base de données: {e}"))
            }
            AuthError::TokenError(e) => {
                response.status(Status::Unauthorized);
                ApiResponse::<u8>::error(format!("Erreur de token: {e}"))
            }
            AuthError::UnknownUser => {
                response.status(Status::Unauthorized);
                ApiResponse::<u8>::error("Utilisateur inconnu".to_string())
            }
        };

        let body_string = rocket::serde::json::to_string(&body).unwrap();
        response.sized_body(body_string.len(), Cursor::new(body_string));
        response.ok()
    }
}
