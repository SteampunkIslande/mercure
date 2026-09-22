mod admin;
mod editgroups;
mod editrun;
mod editusers;
mod generics;
mod home;
mod redirect;
mod showrun;
mod submitrun;

pub use admin::*;
pub use editgroups::*;
pub use editrun::*;
pub use editusers::*;
pub use generics::*;
pub use home::*;
pub use redirect::*;
use rocket::{Response, http::Status, response::Responder};
pub use showrun::*;
pub use submitrun::*;

use crate::templates::{Template, context};

#[derive(Debug, thiserror::Error)]
pub enum FrontendError {
    #[error(transparent)]
    ModelError(#[from] crate::models::ModelError),
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error(transparent)]
    UserError(#[from] anyhow::Error),
    #[error("Erreur logique: {0}")]
    LogicalError(String),
    #[error(transparent)]
    AttemptError(#[from] crate::models::AttemptError),
}

impl<'r> Responder<'r, 'static> for FrontendError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> Result<Response<'static>, Status> {
        let (title, detail) = match &self {
            FrontendError::ModelError(e) => ("Erreur de modèle".to_string(), e.to_string()),
            FrontendError::SqlxError(e) => ("Erreur SQL".to_string(), e.to_string()),
            FrontendError::UserError(e) => ("Erreur utilisateur".to_string(), e.to_string()),
            FrontendError::AttemptError(e) => ("Erreur de tentative".to_string(), e.to_string()),
            FrontendError::LogicalError(e) => ("Erreur logique".to_string(), e.to_string()),
        };

        Template::render(
            "common/error",
            context! {
                title => title,
                h2 => title,
                message => detail,
            },
        )
        .respond_to(req)
    }
}
