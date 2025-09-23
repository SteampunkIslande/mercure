mod guard;

pub use guard::*;

use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Token error: {0}")]
    TokenError(String),
}

impl<'r> Responder<'r, 'static> for AuthError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let mut response = Response::build();

        match self {
            AuthError::InvalidCredentials => response.status(Status::Unauthorized),
            AuthError::DatabaseError(_) => response.status(Status::InternalServerError),
            AuthError::TokenError(_) => response.status(Status::Unauthorized),
        };

        response.ok()
    }
}
