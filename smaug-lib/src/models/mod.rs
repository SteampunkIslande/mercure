pub mod analysis;
pub mod appstate;
pub mod attempt;
pub mod form;
pub mod groups;
pub mod hgrun;
pub mod run_progress;
pub mod user;

use std::convert::Infallible;

pub use analysis::*;
pub use appstate::*;
pub use attempt::*;
pub use form::*;
pub use groups::*;
pub use hgrun::*;
pub use run_progress::*;
pub use user::*;

/// Errors that belong to the model layer as a whole.
///
/// The model layer is the "middle" of the application: its functions return
/// `anyhow::Result`. The few errors listed here are only raised when a caller
/// *knows how to name* the failure, and are therefore reusable inside an
/// `anyhow::Error` chain (that is what lets the edge `Responder`s map them to a
/// specific HTTP status).///
/// These types are the *named* errors of the layer; the functions themselves
/// return `anyhow::Result`. The `InternalError` variant only exists to give the
/// edge errors (FrontendError / AuthError) a single `#[from] anyhow::Error`
/// conversion point; it is not produced by the model layer itself.
#[derive(Debug, wherror::Error)]
pub enum ModelError {
    #[error("Entrée invalide: {0}")]
    InvalidInput(String),
    #[error("{location}: {source}")]
    InternalError {
        #[from]
        source: anyhow::Error,
        location: &'static std::panic::Location<'static>,
    },
}

impl From<Infallible> for ModelError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
