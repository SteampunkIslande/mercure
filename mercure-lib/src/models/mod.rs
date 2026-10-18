pub mod analysis;
pub mod appstate;
pub mod attempt;
pub mod form;
pub mod groups;
pub mod hgrun;
pub mod user;

use std::convert::Infallible;

pub use analysis::*;
pub use appstate::*;
pub use attempt::*;
pub use form::*;
pub use groups::*;
pub use hgrun::*;
pub use user::*;

use crate::pipeline_exec::versionning;

#[derive(Debug, wherror::Error)]
pub enum ModelError {
    #[error("Form serde error from/to json in file {location}: {source}")]
    SerdeJsonError {
        #[from]
        source: serde_json::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("Form serde error from/to yaml in file {location}: {source}")]
    SerdeYamlError {
        #[from]
        source: yaml_serde::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    DatabaseError {
        #[from]
        source: sqlx::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    RunDefinitionError {
        #[from]
        source: hgrun::RunDefinitionError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    GitCheckError {
        #[from]
        source: versionning::GitCheckError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    IOError {
        #[from]
        source: std::io::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    AttemptError {
        #[from]
        source: attempt::AttemptError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    BcryptError {
        #[from]
        source: bcrypt::BcryptError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    FormError {
        #[from]
        source: form::FormDefinitionError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    AnyHowError {
        #[from]
        source: anyhow::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl From<Infallible> for ModelError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
