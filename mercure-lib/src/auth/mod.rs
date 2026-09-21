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
    #[error(transparent)]
    SqliteError(#[from] sqlx::Error),
    #[error(transparent)]
    BcryptError(#[from] bcrypt::BcryptError),
    #[error("No password given")]
    NoPassword,
    #[error(transparent)]
    ModelError(#[from] crate::models::ModelError),
}

impl<'r> Responder<'r, 'static> for AuthError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let mut response = Response::build();
        let body = match self {
            AuthError::InvalidCredentials => {
                response.status(Status::Unauthorized);
                ApiResponse::<()>::error("Mot de passe invalide".to_string())
            }
            AuthError::DatabaseError(e) => {
                response.status(Status::InternalServerError);
                ApiResponse::<()>::error(format!("Erreur de la base de données: {e}"))
            }
            AuthError::TokenError(e) => {
                response.status(Status::Unauthorized);
                ApiResponse::<()>::error(format!("Erreur de token: {e}"))
            }
            AuthError::UnknownUser => {
                response.status(Status::Unauthorized);
                ApiResponse::<()>::error("Utilisateur inconnu".to_string())
            }
            AuthError::SqliteError(e) => {
                response.status(Status::InternalServerError);
                ApiResponse::<()>::error(e.to_string())
            }
            AuthError::BcryptError(e) => {
                response.status(Status::InternalServerError);
                ApiResponse::<()>::error(e.to_string())
            }
            AuthError::NoPassword => {
                response.status(Status::BadRequest);
                ApiResponse::<()>::error(self.to_string())
            }
            AuthError::ModelError(e) => {
                response.status(Status::InternalServerError);
                ApiResponse::<()>::error(format!("Erreur générique: {e}"))
            }
        };

        let body_string = rocket::serde::json::to_string(&body).unwrap();
        response.sized_body(body_string.len(), Cursor::new(body_string));
        response.ok()
    }
}
