use std::path::PathBuf;

use crate::{
    config::{GitWebConfig, MercureConfig, get_mercure_config},
    models::FormIdentification,
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
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
    let MercureConfig { pipelines, .. } = get_mercure_config();

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

            if item.item_type.eq_ignore_ascii_case("file") && is_yaml {
                results.push(item);
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

pub async fn get_file_giteav1(
    base_url: &str,
    owner: &str,
    repo: &str,
    form_identification: &FormIdentification,
) -> Result<String, GitCheckError> {
    let client = Client::new();
    // bioinfo/mercure/raw/branch/new/mercure-lib/Cargo.lock
    let url = match form_identification {
        FormIdentification::GitBranchAndPath {
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
        FormIdentification::GitCommit {
            hash,
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
        FormIdentification::None => {
            return Err(anyhow::format_err!("Identification du formulaire invalide").into());
        }
    };

    Ok(client.get(&url).send().await?.text().await?)
}

pub async fn get_all_branches() -> Result<Vec<String>, GitCheckError> {
    let MercureConfig { pipelines, .. } = get_mercure_config();

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
