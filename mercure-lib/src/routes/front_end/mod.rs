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
mod runtime_config;
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
use rocket::{
    Response,
    http::{Status, uri::error::PathError},
    response::Responder,
};
pub use runtime_config::*;
pub use showrun::*;

use crate::{
    routes::frontend::FrontendError::InvalidPath,
    templates::{Template, context},
};

#[derive(Debug, wherror::Error)]
pub enum FrontendError {
    #[error(transparent)]
    ModelError(#[from] crate::models::ModelError),
    #[error("Erreur logique: {0}")]
    LogicalError(String),
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
    #[error("Accès réservé aux admins")]
    AdminsOnly,
    #[error("Chemin invalide")]
    InvalidPath(String),
}

/// Convert a rocket::...::PathError into a FrontendError because PathError doesn't implement as_dyn_error
impl From<PathError> for FrontendError {
    fn from(value: PathError) -> Self {
        match value {
            PathError::BadChar(c) => {
                InvalidPath(format!("caractère invalide dans le chemin:{}", c))
            }
            PathError::BadStart(c) => {
                InvalidPath(format!("premier caractère invalide dans le chemin:{}", c))
            }
            PathError::BadEnd(c) => {
                InvalidPath(format!("dernier caractère invalide dans le chemin:{}", c))
            }
        }
    }
}

impl<'r> Responder<'r, 'static> for FrontendError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> Result<Response<'static>, Status> {
        let (title, detail) = match &self {
            FrontendError::ModelError(e) => ("Erreur de modèle".to_string(), e.to_string()),
            FrontendError::LogicalError(e) => ("Erreur logique".to_string(), e.to_string()),
            FrontendError::AnyhowError(e) => {
                // 1. The outermost error (first in the chain) becomes the title.
                // anyhow::Error's Display implementation already gives us this.
                let title = format!("Erreur : {}", e);

                // 2. We skip the first error and collect the underlying causes.
                let causes_html = e
                    .chain()
                    .skip(1)
                    .map(|cause| format!("<li>{}</li>", cause))
                    .collect::<Vec<_>>();

                // 3. Format the body with a simple, clean HTML list style.
                let body = if causes_html.is_empty() {
                    String::new() // Or perhaps "<p>Aucun détail supplémentaire.</p>".to_string()
                } else {
                    format!(
                        "<p><strong>Causée par :</strong></p><ul>{}</ul>",
                        causes_html.join("")
                    )
                };

                (title, body)
            }
            FrontendError::AdminsOnly => {
                return Template::render("error/admin_only", context! {}).respond_to(req);
            }
            FrontendError::InvalidPath(s) => (
                "Chemin invalide".to_string(),
                format!("Erreur de chemin: {s}"),
            ),
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
