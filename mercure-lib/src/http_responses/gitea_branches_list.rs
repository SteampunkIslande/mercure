use reqwest::get;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use super::ExternalResponsesErrors;

pub type BranchesList = Vec<BranchDef>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchDef {
    pub name: String,
    pub commit: Commit,
    pub protected: bool,
    pub required_approvals: i64,
    pub enable_status_check: bool,
    pub status_check_contexts: Vec<Value>,
    pub user_can_push: bool,
    pub user_can_merge: bool,
    pub effective_branch_protection_name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub message: String,
    pub url: String,
    pub author: Author,
    pub committer: Committer,
    pub verification: Verification,
    pub timestamp: String,
    pub added: Value,
    pub removed: Value,
    pub modified: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub email: String,
    pub username: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Committer {
    pub name: String,
    pub email: String,
    pub username: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verification {
    pub verified: bool,
    pub reason: String,
    pub signature: String,
    pub signer: Value,
    pub payload: String,
}

/// Returns a list of branches in a repository, as served from a given gitea host
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
pub async fn list_branches(
    hostname: &str,
    owner: &str,
    repo: &str,
) -> Result<Vec<String>, ExternalResponsesErrors> {
    let response = get(format!("{hostname}/api/v1/repos/{owner}/{repo}/branches")).await?;
    Ok(
        serde_json::from_str::<BranchesList>(&response.text().await?)?
            .into_iter()
            .map(|b| b.name)
            .collect(),
    )
}
