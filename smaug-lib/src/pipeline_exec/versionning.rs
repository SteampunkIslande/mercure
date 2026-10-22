use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    config::{GitWebConfig, SmaugConfig, get_smaug_config},
    models::FileIdentification,
};

use log::{error, info};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio::process::Command;
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
}

#[derive(Debug, Deserialize)]
struct GiteaCommitResponse {
    id: String,
    // the rest of the commit is not needed for BranchInfo
    #[serde(flatten)]
    _rest: serde::de::IgnoredAny,
}

#[derive(Debug, Deserialize)]
pub struct GiteaBranchResponse {
    name: String,
    commit: GiteaCommitResponse,
    // Gitea also returns effective_branch_protection_name, protected,... we ignore them here
    #[serde(default)]
    _rest: serde::de::IgnoredAny,
}

pub struct BranchInfo {
    pub name: String,
    pub last_commit: String,
}

impl BranchInfo {
    pub fn from_gitea(b: GiteaBranchResponse) -> Self {
        Self {
            name: b.name.clone(),
            last_commit: b.commit.id.clone(),
        }
    }
}

pub async fn get_latest_commit(
    branch_name: Option<&str>,
) -> Result<Vec<BranchInfo>, GitCheckError> {
    let SmaugConfig { pipelines, .. } = get_smaug_config();

    match &pipelines {
        GitWebConfig::GiteaV1 {
            base_url,
            owner,
            repo,
        } => {
            let url = format!("{base_url}/api/v1/repos/{owner}/{repo}/branches");
            let branches: Vec<GiteaBranchResponse> = reqwest::get(&url)
                .await?
                .json::<Vec<GiteaBranchResponse>>()
                .await?;

            Ok(branches
                .into_iter()
                .map(|b| BranchInfo::from_gitea(b))
                .filter(|info| branch_name.map(|n| n == &info.name).unwrap_or(true))
                .collect())
        }

        GitWebConfig::GitlabV4 { base_url: _, id: _ } => Err(GitCheckError::Unsupported(
            "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
        )),
    }
}

#[derive(Debug, Deserialize)]
pub struct DirContentsItem {
    pub name: String,
    pub path: PathBuf,
    #[serde(rename = "type")]
    pub item_type: String,
    pub size: Option<u64>,
    // les autres champs ne sont pas nécessaires ici
}

#[derive(Debug, Deserialize)]
pub struct DirContentsResponse {
    pub dir_contents: Vec<DirContentsItem>,
}

#[derive(Debug, Deserialize)]
pub struct FileContentsResponse {
    pub file_contents: DirContentsItem,
}

pub async fn list_yaml_forms_giteav1(
    base_url: &str,
    owner: &str,
    repo: &str,
    branch_name: &str,
) -> Result<Vec<DirContentsItem>, GitCheckError> {
    let client = Client::new();
    let mut results = Vec::new();
    let mut stack = vec![".".to_string()];

    while let Some(current_path) = stack.pop() {
        let url = format!("{base_url}/api/v1/repos/{owner}/{repo}/contents-ext/{current_path}");

        let resp = client
            .get(&url)
            .query(&[("ref", branch_name)])
            .send()
            .await?;

        if !resp.status().is_success() {
            continue;
        }

        // Idiomatic let-else pattern replaces the verbose match block
        let Ok(body) = resp.json::<DirContentsResponse>().await else {
            continue;
        };

        for item in body.dir_contents {
            let is_yaml = item.name.ends_with(".yml") || item.name.ends_with(".yaml");

            if item.item_type.eq_ignore_ascii_case("file")      // Is a file
                && is_yaml                                      // Is a yaml file
                && item                                         // Has a folder named .forms in its parents
                    .path
                    .components()
                    .find(|c| c.as_os_str().eq_ignore_ascii_case(".forms"))
                    .is_some()
            {
                results.push(item); // Then it must be a valid form :)
            } else if item.item_type.eq_ignore_ascii_case("dir") {
                // Cleaner path handling without OS-string lossy conversions
                let next_path = match item.path.to_str() {
                    Some("") | None => format!("{current_path}/{}", item.name),
                    Some(valid_path) => valid_path.to_string(),
                };
                stack.push(next_path);
            }
        }
    }

    Ok(results)
}

async fn is_file_giteav1(
    base_url: &str,
    owner: &str,
    repo: &str,
    form_identification: &FileIdentification,
) -> Result<bool, GitCheckError> {
    let client = Client::new();
    let url = match form_identification {
        FileIdentification::GitBranchAndPath { branch, file_path } => {
            format!(
                "{}/api/v1/repos/{}/{}/contents-ext/{}?ref={}",
                base_url,
                owner,
                repo,
                file_path.display(),
                branch
            )
        }
        FileIdentification::GitCommit {
            commit_hash,
            file_path,
        } => {
            format!(
                "{}/api/v1/repos/{}/{}/contents-ext/{}?ref={}",
                base_url,
                owner,
                repo,
                file_path.display(),
                commit_hash
            )
        }
        FileIdentification::None => {
            return Err(anyhow::format_err!("Identification du formulaire invalide").into());
        }
    };
    Ok(client
        .get(&url)
        .send()
        .await?
        .json::<FileContentsResponse>()
        .await?
        .file_contents
        .item_type
        .as_str()
        == "file")
}

pub async fn get_file_giteav1(
    base_url: &str,
    owner: &str,
    repo: &str,
    form_identification: &FileIdentification,
) -> Result<String, GitCheckError> {
    let client = Client::new();

    if !is_file_giteav1(base_url, owner, repo, form_identification).await? {
        return Err(anyhow::format_err!("Ceci n'est pas un fichier!").into());
    }

    // bioinfo/smaug/raw/branch/new/smaug-lib/Cargo.lock
    let url = match form_identification {
        FileIdentification::GitBranchAndPath {
            branch,
            file_path: path,
        } => {
            format!(
                "{}/{}/{}/raw/branch/{}/{}",
                base_url,
                owner,
                repo,
                branch,
                path.display()
            )
        }
        FileIdentification::GitCommit {
            commit_hash: hash,
            file_path: path,
        } => {
            format!(
                "{}/{}/{}/raw/commit/{}/{}",
                base_url,
                owner,
                repo,
                hash,
                path.display()
            )
        }
        FileIdentification::None => {
            return Err(anyhow::format_err!("Identification du formulaire invalide").into());
        }
    };

    Ok(client.get(&url).send().await?.text().await?)
}

pub async fn get_all_branches() -> Result<Vec<String>, GitCheckError> {
    let SmaugConfig { pipelines, .. } = get_smaug_config();

    match &pipelines {
        GitWebConfig::GiteaV1 {
            base_url,
            owner,
            repo,
        } => Ok(serde_json::from_str::<Value>(
            &reqwest::get(&format!("{base_url}/api/v1/repos/{owner}/{repo}/branches"))
                .await?
                .text()
                .await?,
        )?
        .as_array()
        .ok_or(anyhow::anyhow!("Gitea should have responded with an array"))?
        .iter()
        .filter_map(|v| Some(v["name"].as_str()?.to_string()))
        .collect()),

        GitWebConfig::GitlabV4 { base_url: _, id: _ } => {
            todo!("L'API Gitlab V4 n'est pas encore prise en charge!")
        }
    }
}

pub async fn git_clone_to(
    path: &Path,
    file_identification: &FileIdentification,
) -> Result<(), GitCheckError> {
    let SmaugConfig { pipelines, .. } = get_smaug_config();

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
            GitCheckError::CommandFailed(e.to_string()) // Adjust to your actual error variants
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
            return Err(GitCheckError::CommandFailed(format!(
                "Checkout failed: {}",
                status
            )));
        }
        Ok(Err(e)) => {
            error!("Failed to wait for git checkout process: {}", e);
            return Err(GitCheckError::CommandFailed(e.to_string()));
        }
        Err(_) => {
            error!("Git checkout timed out after 5 seconds.");
            let _ = checkout_child.kill().await;
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
