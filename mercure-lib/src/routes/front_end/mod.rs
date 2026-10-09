mod admin;
mod editgroups;
mod editrun;
mod editusers;
mod forms;
mod generics;
mod home;
mod newgroup;
mod newrun;
mod redirect;
mod showrun;

pub use admin::*;
pub use editgroups::*;
pub use editrun::*;
pub use editusers::*;
pub use forms::*;
pub use generics::*;
pub use home::*;
pub use newgroup::*;
pub use newrun::*;
pub use redirect::*;
use rocket::{Response, http::Status, response::Responder};
pub use showrun::*;

use crate::templates::{Template, context};

#[derive(Debug, thiserror::Error)]
pub enum FrontendError {
    #[error(transparent)]
    ModelError(#[from] crate::models::ModelError),
    #[error("Erreur logique: {0}")]
    LogicalError(String),
    #[error(transparent)]
    GenericError(#[from] anyhow::Error),
}

impl<'r> Responder<'r, 'static> for FrontendError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> Result<Response<'static>, Status> {
        let (title, detail) = match &self {
            FrontendError::ModelError(e) => ("Erreur de modèle".to_string(), e.to_string()),
            FrontendError::LogicalError(e) => ("Erreur logique".to_string(), e.to_string()),
            FrontendError::GenericError(e) => ("Erreur générique".to_string(), e.to_string()),
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
