use wherror::Error;

/// Errors specific to the git repository layer.
///
/// Only the errors that the git layer *knows how to name* live here: they are
/// the ones a caller can meaningfully match on (`NotAFile`, `Unsupported`,
/// `CommandFailed`). The git helpers return `Result<_, GitCheckError>`, so the
/// foreign errors (I/O, serde, reqwest, …) they meet are raised as
/// `anyhow::Error` with a message and carried by
/// [`GitCheckError::InternalError`], the unique `#[from]` variant of this enum.
#[derive(Error, Debug)]
pub enum GitCheckError {
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    CommandFailed(String),
    #[error("{0}")]
    NotAFile(String),
    #[error("{location}: {source}")]
    InternalError {
        #[from]
        source: anyhow::Error,
        location: &'static std::panic::Location<'static>,
    },
}
