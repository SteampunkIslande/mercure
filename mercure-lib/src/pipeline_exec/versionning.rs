use crate::config::{GitWebConfig, MercureConfig, get_mercure_config};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitCheckError {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("La branche {0} n'existe pas!")]
    NoSuchBranch(String),
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
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
    pub path: String,
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

    let url = format!(
        "{}/api/v1/repos/{}/{}/contents-ext/.forms",
        base_url, owner, repo
    );

    let resp = client
        .get(&url)
        .query(&[("ref", branch_name)])
        .send()
        .await?
        .error_for_status()?;

    let body = resp.json::<DirContentsResponse>().await?;

    // filtre uniquement les fichiers yml / yaml
    let filtered = body
        .dir_contents
        .into_iter()
        .filter(|item| {
            item.item_type.eq_ignore_ascii_case("file")
                && (item.name.ends_with(".yml") || item.name.ends_with(".yaml"))
        })
        .collect::<Vec<_>>();

    Ok(filtered)
}

pub async fn get_file_giteav1(
    base_url: &str,
    owner: &str,
    repo: &str,
    branch_name: &str,
    path: &str,
) -> Result<String, GitCheckError> {
    let client = Client::new();
    // bioinfo/mercure/raw/branch/new/mercure-lib/Cargo.lock
    let url = format!(
        "{}/{}/{}/raw/branch/{}/{}",
        base_url, owner, repo, branch_name, path
    );
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
