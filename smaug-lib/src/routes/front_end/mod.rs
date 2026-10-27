mod admin;
mod data;
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
pub use data::*;
pub use editgroups::*;
pub use editrun::*;
pub use editusers::*;
pub use forms::*;
pub use generics::*;
pub use home::*;
pub use newgroup::*;
pub use newrun::*;
pub use redirect::*;
pub use runtime_config::*;
pub use showrun::*;

use rocket::{Response, http::Status, response::Responder};

use crate::models::{
    AnalysisStateMachineError, AttemptError, FormDefinitionError, ModelError, RunDefinitionError,
};
use crate::templates::{Template, context};
use crate::utils::SamplesheetError;

/// What the browser gets when a page cannot be rendered.
///
/// This is an edge error: routes only ever see a [`FrontendError`] built from an
/// `anyhow::Error`. The `anyhow::Error` variant is therefore the **unique**
/// `#[from]` variant: the *cause* is inspected there (with `downcast`) to decide
/// what to display, and the reported location is the one where the error actually
/// occurred, carried by wherror.
#[derive(Debug, wherror::Error)]
pub enum FrontendError {
    /// The user is authenticated but not allowed to see this page.
    #[error("Accès réservé aux admins")]
    AdminsOnly,
    /// The run references a form that no longer exists in the pipelines repository.
    #[error("Le formulaire '{file_path}' sur la branche '{branch}' n'existe plus")]
    FormNotFound { branch: String, file_path: String },
    #[error("Erreur dans le fichier {location}: {source}")]
    AnyhowError {
        #[from]
        source: anyhow::Error,
        location: &'static std::panic::Location<'static>,
    },
}

/// Extracts the user-facing message from a named model error.
///
/// The model layer returns `anyhow::Result`; when a route surfaces one of its
/// errors, the named error carries a message written for the user. Any other
/// error falls back to the full context chain.
fn cause_message(error: &anyhow::Error) -> String {
    if let Some(cause) = error.downcast_ref::<ModelError>() {
        return cause.to_string();
    }
    if let Some(cause) = error.downcast_ref::<RunDefinitionError>() {
        return cause.to_string();
    }
    if let Some(cause) = error.downcast_ref::<AnalysisStateMachineError>() {
        return cause.to_string();
    }
    if let Some(cause) = error.downcast_ref::<FormDefinitionError>() {
        return cause.to_string();
    }
    if let Some(cause) = error.downcast_ref::<AttemptError>() {
        return cause.to_string();
    }
    if let Some(cause) = error.downcast_ref::<SamplesheetError>() {
        return cause.to_string();
    }
    format!("{error:#}")
}

/// Renders the error page shown to the user. Free-standing because the
/// `Responder` needs a `Template`, which the error type itself must not depend on.
///
/// `common/error.html.j2` renders the message unescaped (`| safe`), and the cause
/// chain contains user-provided values (form paths, variable values), so the
/// detail is HTML-escaped here.
fn error_page(title: String, detail: &str) -> Template {
    Template::render(
        "common/error",
        context! {
            title => title,
            h2 => title,
            message => html_escape::encode_text(detail).to_string(),
        },
    )
}

impl<'r> Responder<'r, 'static> for FrontendError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> Result<Response<'static>, Status> {
        let page = match &self {
            FrontendError::AdminsOnly => error_page(
                "Accès protégé".to_string(),
                "Seuls les administrateurs peuvent accéder à cette page.",
            ),
            FrontendError::FormNotFound { branch, file_path } => error_page(
                "Formulaire introuvable".to_string(),
                &format!(
                    "Le formulaire '{file_path}' sur la branche '{branch}' n'existe plus. Ce run ne peut donc plus être édité."
                ),
            ),
            FrontendError::AnyhowError { source, location } => {
                // The location and the full chain are for the server logs only.
                log::error!("Erreur page dans le fichier {location}: {source:#}");
                error_page(
                    "Une erreur est survenue".to_string(),
                    &cause_message(source),
                )
            }
        };

        page.respond_to(req)
    }
}
