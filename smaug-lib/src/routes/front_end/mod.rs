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
pub use runtime_config::*;
pub use showrun::*;

use rocket::{Response, http::Status, response::Responder};

use crate::models::ModelError;
use crate::templates::{Template, context};

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
    #[error("Erreur dans le fichier {location}: {source}")]
    AnyhowError {
        #[from]
        source: anyhow::Error,
        location: &'static std::panic::Location<'static>,
    },
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
            FrontendError::AnyhowError { source, location } => {
                // The location and the full chain are for the server logs only.
                log::error!("Erreur page dans le fichier {location}: {source:#}");
                let cause = source
                    .downcast_ref::<ModelError>()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| format!("{source:#}"));
                error_page(format!("Erreur dans le fichier {location}"), &cause)
            }
        };

        page.respond_to(req)
    }
}
