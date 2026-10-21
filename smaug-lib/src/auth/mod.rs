mod auth_guard;

use std::io::Cursor;

use super::routes::ApiResponse;

use crate::models;

pub use auth_guard::*;

use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

#[derive(Debug, wherror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Unknown user")]
    UnknownUser,
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Token error: {0}")]
    TokenError(String),
    #[error("{location}: {source}")]
    SqliteError {
        #[from]
        source: sqlx::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    BcryptError {
        #[from]
        source: bcrypt::BcryptError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("No password given")]
    NoPassword,
    #[error("Erreur dans le fichier {location}: {source}")]
    ModelError {
        #[from]
        source: models::ModelError,
        location: &'static std::panic::Location<'static>,
    },
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
            AuthError::SqliteError { source, location } => {
                response.status(Status::InternalServerError);
                ApiResponse::<()>::error(format!("Erreur dans le fichier {location}: {source}"))
            }
            AuthError::BcryptError { source, location } => {
                response.status(Status::InternalServerError);
                ApiResponse::<()>::error(format!("Erreur dans le fichier {location}: {source}"))
            }
            AuthError::NoPassword => {
                response.status(Status::BadRequest);
                ApiResponse::<()>::error(self.to_string())
            }
            AuthError::ModelError { source, location } => {
                response.status(Status::InternalServerError);
                ApiResponse::<()>::error(format!("Erreur dans le fichier {location}: {source}"))
            }
        };

        let body_string = rocket::serde::json::to_string(&body).unwrap();
        response.sized_body(body_string.len(), Cursor::new(body_string));
        response.ok()
    }
}
