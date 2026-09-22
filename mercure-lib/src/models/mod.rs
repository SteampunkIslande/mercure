pub mod analysis;
pub mod attempt;
pub mod form;
pub mod groups;
pub mod hgrun;
pub mod user;

use std::convert::Infallible;

pub use analysis::*;
pub use attempt::*;
pub use form::*;
pub use groups::*;
pub use hgrun::*;
pub use user::*;

use crate::{models, pipeline_exec::versionning};

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("Form error: {0}")]
    FormError(String),
    #[error("Form serde error from/to json: {0}")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Form serde error from/to yaml: {0}")]
    SerdeYamlError(#[from] yaml_serde::Error),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    RunDefinitionError(#[from] hgrun::RunDefinitionError),
    #[error(transparent)]
    GitCheckError(#[from] versionning::GitCheckError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    AttemptError(#[from] models::AttemptError),
    #[error(transparent)]
    BcryptError(#[from] bcrypt::BcryptError),
}

impl From<Infallible> for ModelError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
