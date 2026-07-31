use std::fmt::Display;

use super::ExternalResponsesErrors;
use reqwest::get;

pub enum Revision<'a> {
    Branch(&'a str),
    Tag(&'a str),
    Commit(&'a str),
}

impl<'a> Display for Revision<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Revision::Branch(b) => {
                write!(f, "/branch/{}", b)
            }
            Revision::Tag(t) => {
                write!(f, "/tag/{}", t)
            }
            Revision::Commit(h) => {
                write!(f, "/commit/{}", h)
            }
        }
    }
}

/// Returns raw file content as a string
///
/// # Arguments
/// - `hostname`: URL to a gitea host (with leading http)
/// - `owner`: Repository owner
/// - `repo`: Repository name
///
/// # Returns
/// `Result<Vec<String>, ExternalResponsesErrors>`. On success, all the existing branch names in this repository
/// # Errors
/// - `hostname` is unreachable (not responding, or does not exist)
/// - `owner` does not exist
/// - `repo` does not exist or it requires login
pub async fn get_file_content<'a>(
    base_url: &str,
    owner: &str,
    repo_name: &str,
    revision: &'_ Revision<'_>,
    file_name: &str,
) -> Result<String, ExternalResponsesErrors> {
    Ok(get(format!(
        "{base_url}/{owner}/{repo_name}/raw/{revision}/{file_name}"
    ))
    .await?
    .text()
    .await?)
}
