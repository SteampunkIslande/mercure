mod error;
mod gitea;
use crate::{config::GitWebConfig, models::FileIdentification};
use anyhow::Context;
pub use error::GitCheckError;
use serde::Deserialize;
use serde_json::Value;
use std::{path::Path, time::Duration};

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
            return Err(GitCheckError::Unsupported(
                "Impossible de cloner un dépôt sans branche ni commit".into(),
            ));
        }
    };

    let (clone_url, repo_name) = match pipelines {
        GitWebConfig::GiteaV1 {
            base_url,
            owner,
            repo,
        } => (format!("{base_url}/{owner}/{repo}.git"), repo),
        GitWebConfig::GitlabV4 { .. } => {
            return Err(GitCheckError::Unsupported(
                "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
            ));
        }
    };

    let repo_path = path.join(&repo_name);
    if repo_path.exists() {
        tokio::fs::remove_dir_all(&repo_path)
            .await
            .with_context(|| {
                format!(
                    "Impossible de supprimer le dossier pipelines de la tentative précédente ({})",
                    repo_path.display()
                )
            })?;
    }
    log::info!("Cloning {} into {}", clone_url, repo_path.display());

    let mut clone_child = Command::new("git")
        .arg("clone")
        .arg(&clone_url)
        .arg(&repo_name)
        .current_dir(path)
        .spawn()
        .with_context(|| format!("Impossible de lancer `git clone {clone_url}`"))?;

    match tokio::time::timeout(Duration::from_secs(30), clone_child.wait()).await {
        Ok(Ok(status)) if status.success() => log::info!("Git clone successful."),
        Ok(Ok(status)) => {
            return Err(GitCheckError::CommandFailed(format!(
                "Clone failed: {}",
                status
            )));
        }
        Ok(Err(e)) => {
            return Err(e).with_context(|| {
                format!("Erreur en attendant la fin de `git clone {clone_url}`")
            })?;
        }
        Err(_) => {
            let _ = clone_child.kill().await;
            // Try to clean up if repo was partially created
            let _ = tokio::fs::remove_dir_all(&repo_path).await;
            return Err(GitCheckError::CommandFailed("Clone timeout".into()));
        }
    }

    log::info!("Checking out '{}' in {}", checkout, repo_path.display());

    let mut checkout_child = Command::new("git")
        .arg("checkout")
        .arg(&checkout)
        .current_dir(&repo_path) // Fixed: Must run inside the cloned repository
        .spawn()
        .with_context(|| {
            format!(
                "Impossible de lancer `git checkout {checkout}` dans {}",
                repo_path.display()
            )
        })?;

    match tokio::time::timeout(Duration::from_secs(5), checkout_child.wait()).await {
        Ok(Ok(status)) if status.success() => log::info!("Git checkout successful."),
        Ok(Ok(status)) => {
            let _ = tokio::fs::remove_dir_all(&repo_path).await;
            return Err(GitCheckError::CommandFailed(format!(
                "Checkout failed: {}",
                status
            )));
        }
        Ok(Err(e)) => {
            let _ = tokio::fs::remove_dir_all(&repo_path).await;
            return Err(e).with_context(|| {
                format!("Erreur en attendant la fin de `git checkout {checkout}`")
            })?;
        }
        Err(_) => {
            let _ = checkout_child.kill().await;
            let _ = tokio::fs::remove_dir_all(&repo_path).await;
            return Err(GitCheckError::CommandFailed("Checkout timeout".into()));
        }
    }

    let git_dir = repo_path.join(".git");
    log::info!("Removing {:?}", git_dir);

    tokio::fs::remove_dir_all(&git_dir)
        .await
        .with_context(|| format!("Impossible de supprimer le dossier {}", git_dir.display()))?;

    log::info!("Successfully prepped repository at {:?}", repo_path);
    Ok(())
}

pub async fn get_latest_commit(
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

            let body = reqwest::get(&url)
                .await
                .with_context(|| format!("Impossible d'interroger l'API Gitea ({url})"))?
                .text()
                .await
                .with_context(|| format!("Impossible de lire la réponse de l'API Gitea ({url})"))?;
            let commit_id = serde_json::from_str::<Value>(&body).with_context(|| {
                format!("Réponse inattendue de l'API Gitea pour la branche {branch_name}")
            })?["commit"]["id"]
                .as_str()
                .map(|id| id.to_string())
                .ok_or_else(|| {
                    GitCheckError::CommandFailed(format!(
                        "Cannot get latest commit ID for branch {}",
                        branch_name
                    ))
                })?;
            Ok(commit_id)
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
            let resp = reqwest::get(&url)
                .await
                .with_context(|| format!("Impossible de lister les branches ({url})"))?;
            if !resp.status().is_success() {
                return Err(GitCheckError::CommandFailed(format!(
                    "Failed to list branches: {}",
                    resp.status()
                )));
            }
            let v = resp
                .json::<Value>()
                .await
                .with_context(|| format!("Réponse inattendue de l'API Gitea ({url})"))?;
            let arr = v.as_array().ok_or_else(|| {
                GitCheckError::CommandFailed(
                    "Gitea should have responded with an array".to_string(),
                )
            })?;
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
    file_identification: &FileIdentification,
) -> Result<String, GitCheckError> {
    match pipelines {
        GitWebConfig::GiteaV1 {
            base_url,
            owner,
            repo,
        } => gitea::git_get_file(base_url, owner, repo, file_identification).await,
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
