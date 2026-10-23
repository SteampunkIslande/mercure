use wherror::Error;

#[derive(Error, Debug)]
pub enum GitCheckError {
    #[error("{location}: {source}")]
    ReqwestError {
        #[from]
        source: reqwest::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    SerdeJsonError {
        #[from]
        source: serde_json::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("La branche {0} n'existe pas!")]
    NoSuchBranch(String),
    #[error("{location}: {source}")]
    AnyhowError {
        #[from]
        source: anyhow::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{0}")]
    Unsupported(String),
    #[error("{location}:{source}")]
    IOError {
        #[from]
        source: std::io::Error,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{0}")]
    CommandFailed(String),
    #[error("{0}")]
    NotAFile(String),
}
