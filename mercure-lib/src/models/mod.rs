pub mod analysis;
pub mod attempt;
pub mod groups;
pub mod hgrun;
pub mod user;

use std::convert::Infallible;

pub use analysis::*;
pub use attempt::*;
pub use groups::*;
pub use hgrun::*;
pub use user::*;

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("Form error: {0}")]
    FormError(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    RunDefinitionError(#[from] hgrun::RunDefinitionError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

impl From<Infallible> for ModelError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
