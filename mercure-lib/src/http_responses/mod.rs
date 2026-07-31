pub mod gitea_branches_list;
pub use gitea_branches_list::*;

pub mod gitea_raw_file;
pub use gitea_raw_file::*;

use thiserror::Error;

/// This module provides convenient functions to talk to other web services through JSON APIs.
/// Each sub-module defines its own datatypes that parse these responses

#[derive(Debug, Error)]
pub enum ExternalResponsesErrors {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}
