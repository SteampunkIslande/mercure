pub mod analysis;
pub mod attempt;
pub mod form;
pub mod groups;
pub mod hgrun;
pub mod user;

pub use analysis::*;
pub use attempt::*;
pub use form::*;
pub use groups::*;
pub use hgrun::*;
pub use user::*;

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("Form error: {0}")]
    FormError(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
