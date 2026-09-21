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
}

impl<'r> Responder<'r, 'static> for FrontendError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> Result<Response<'static>, Status> {
        let title = match &self {
            FrontendError::ModelError(_) => "Erreur de la base de données",
        };

        let h2 = self.to_string();
        let message = self.to_string();

        Template::render(
            "common/error",
            context! {
                title => title,
                h2 => h2,
                message => message,
            },
        )
        .respond_to(req)
    }
}
