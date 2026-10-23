mod gitea;
use crate::{config::GitWebConfig, models::FileIdentification};
use serde::Deserialize;
use serde_json::Value;
use std::{path::Path, time::Duration};
mod error;
pub use error::GitCheckError;

use log::{error, info};
use std::path::PathBuf;
use tokio::process::Command;

/// A struct holding everything relevant about a file in a git repository
///
/// # Fields
///
/// - `name` (`String`) - Only the file name, with its extension
/// - `path` (`PathBuf`) - Relative path of the file in the repository
/// - `#[serde(rename = "type")] item_type` (`String`) - File, Directory, or Symlink
/// - `content` (`String`) - The file's content
#[derive(Debug, Deserialize)]
pub struct DirContentsItem {
    pub name: String,
    pub path: PathBuf,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(skip)]
    pub content: String,
}

pub async fn git_clone_to(
    path: &Path,
    file_identification: &FileIdentification,
    pipelines: GitWebConfig,
) -> Result<(), GitCheckError> {
    let checkout = match file_identification {
        FileIdentification::GitBranchAndPath { branch, .. } => branch.to_string(),
        FileIdentification::GitCommit { commit_hash, .. } => commit_hash.to_string(),
        FileIdentification::None => {
            error!("Clone aborted: No branch or commit provided.");
            return Err(GitCheckError::NoSuchBranch("(none)".to_string()));
        }
    };

    let (clone_url, repo_name) = match pipelines {
        GitWebConfig::GiteaV1 {
            base_url,
            owner,
            repo,
        } => (format!("{base_url}/{owner}/{repo}.git"), repo),
        GitWebConfig::GitlabV4 { .. } => {
            error!("Clone aborted: Gitlab V4 API is not yet supported.");
            return Err(GitCheckError::Unsupported(
                "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
            ));
        }
    };

    let repo_path = path.join(&repo_name);
    info!("Cloning {} into {}", clone_url, repo_path.display());

    let mut clone_child = Command::new("git")
        .arg("clone")
        .arg(&clone_url)
        .arg(&repo_name)
        .current_dir(path)
        .spawn()
        .map_err(|e| {
            error!("Failed to spawn git clone process: {}", e);
            GitCheckError::CommandFailed(e.to_string())
        })?;

    match tokio::time::timeout(Duration::from_secs(30), clone_child.wait()).await {
        Ok(Ok(status)) if status.success() => info!("Git clone successful."),
        Ok(Ok(status)) => {
            error!("Git clone failed with exit status: {}", status);
            return Err(GitCheckError::CommandFailed(format!(
                "Clone failed: {}",
                status
            )));
        }
        Ok(Err(e)) => {
            error!("Failed to wait for git clone process: {}", e);
            return Err(GitCheckError::CommandFailed(e.to_string()));
        }
        Err(_) => {
            error!("Git clone timed out after 30 seconds.");
            let _ = clone_child.kill().await;
            // Try to clean up if repo was partially created
            let _ = tokio::fs::remove_dir_all(&repo_path).await;
            return Err(GitCheckError::CommandFailed("Clone timeout".into()));
        }
    }

    info!("Checking out '{}' in {}", checkout, repo_path.display());

    let mut checkout_child = Command::new("git")
        .arg("checkout")
        .arg(&checkout)
        .current_dir(&repo_path) // Fixed: Must run inside the cloned repository
        .spawn()
        .map_err(|e| {
            error!("Failed to spawn git checkout process: {}", e);
            GitCheckError::CommandFailed(e.to_string())
        })?;

    match tokio::time::timeout(Duration::from_secs(5), checkout_child.wait()).await {
        Ok(Ok(status)) if status.success() => info!("Git checkout successful."),
        Ok(Ok(status)) => {
            error!("Git checkout failed with exit status: {}", status);
            let _ = tokio::fs::remove_dir_all(&repo_path).await;
            return Err(GitCheckError::CommandFailed(format!(
                "Checkout failed: {}",
                status
            )));
        }
        Ok(Err(e)) => {
            error!("Failed to wait for git checkout process: {}", e);
            let _ = tokio::fs::remove_dir_all(&repo_path).await;
            return Err(GitCheckError::CommandFailed(e.to_string()));
        }
        Err(_) => {
            error!("Git checkout timed out after 5 seconds.");
            let _ = checkout_child.kill().await;
            let _ = tokio::fs::remove_dir_all(&repo_path).await;
            return Err(GitCheckError::CommandFailed("Checkout timeout".into()));
        }
    }

    let git_dir = repo_path.join(".git");
    info!("Removing {:?}", git_dir);

    if let Err(e) = tokio::fs::remove_dir_all(&git_dir).await {
        error!("Failed to remove .git directory at {:?}: {}", git_dir, e);
        return Err(GitCheckError::CommandFailed(format!("FS error: {}", e)));
    }

    info!("Successfully prepped repository at {:?}", repo_path);
    Ok(())
}

pub async fn get_latest_commit<'a>(
    branch_name: &str,
    pipelines: &GitWebConfig,
) -> Result<String, GitCheckError> {
    match pipelines {
        GitWebConfig::GiteaV1 {
            base_url,
            owner,
            repo,
        } => {
            let url = format!("{base_url}/api/v1/repos/{owner}/{repo}/branches/{branch_name}");

            Ok(
                serde_json::from_str::<Value>(&reqwest::get(&url).await?.text().await?)?["commit"]
                    ["id"]
                    .as_str()
                    .ok_or(anyhow::format_err!(
                        "Cannot get latest commit ID for branch {}",
                        branch_name
                    ))?
                    .to_string(),
            )
        }

        GitWebConfig::GitlabV4 { base_url: _, id: _ } => Err(GitCheckError::Unsupported(
            "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
        )),
    }
}

pub async fn get_all_branches(pipelines: &GitWebConfig) -> Result<Vec<String>, GitCheckError> {
    match pipelines {
        GitWebConfig::GiteaV1 {
            base_url,
            owner,
            repo,
        } => {
            let url = format!("{base_url}/api/v1/repos/{owner}/{repo}/branches");
            let resp = reqwest::get(&url).await?;
            if !resp.status().is_success() {
                return Err(
                    anyhow::format_err!("Failed to list branches: {}", resp.status()).into(),
                );
            }
            let v = resp.json::<Value>().await?;
            let arr = v
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Gitea should have responded with an array"))?;
            Ok(arr
                .iter()
                .filter_map(|v| v["name"].as_str().map(|s| s.to_string()))
                .collect())
        }

        GitWebConfig::GitlabV4 { .. } => Err(GitCheckError::Unsupported(
            "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
        )),
    }
}

pub async fn git_get_file(
    pipelines: &GitWebConfig,
    form_identification: &FileIdentification,
) -> Result<String, GitCheckError> {
    match pipelines {
        GitWebConfig::GiteaV1 {
            base_url,
            owner,
            repo,
        } => gitea::git_get_file(base_url, owner, repo, form_identification).await,
        GitWebConfig::GitlabV4 { .. } => Err(GitCheckError::Unsupported(
            "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
        )),
    }
}

pub async fn list_yaml_forms(
    pipelines: &GitWebConfig,
    branch_name: &str,
) -> Result<Vec<DirContentsItem>, GitCheckError> {
    match pipelines {
        GitWebConfig::GiteaV1 {
            base_url,
            owner,
            repo,
        } => gitea::list_yaml_forms(base_url, owner, repo, branch_name).await,
        GitWebConfig::GitlabV4 { .. } => Err(GitCheckError::Unsupported(
            "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
        )),
    }
}
