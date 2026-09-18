use crate::config::{GitWebConfig, MercureConfig, get_mercure_config};
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
}

pub struct BranchInfo {
    pub name: String,
    pub last_commit: String,
}

impl BranchInfo {
    pub fn from_str_tuple<T, U>(tuple: (T, U)) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        Self {
            name: tuple.0.as_ref().to_string(),
            last_commit: tuple.1.as_ref().to_string(),
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
        } => Ok(serde_json::from_str::<Value>(
            &reqwest::get(&format!("{base_url}/api/v1/repos/{owner}/{repo}/branches"))
                .await?
                .text()
                .await?,
        )?
        .as_array()
        .ok_or(anyhow::anyhow!("Gitea should have responded with an array"))?
        .iter()
        .filter_map(|v| {
            Some((
                v["name"].as_str()?.to_string(),
                v["commit"]["id"].as_str()?.to_string(),
            ))
        })
        .filter(|v| branch_name.as_ref().is_none_or(|n| n == &v.0))
        .map(BranchInfo::from_str_tuple)
        .collect()),

        GitWebConfig::GitlabV4 { base_url: _, id: _ } => {
            todo!("L'API Gitlab V4 n'est pas encore prise en charge!")
        }
    }
}
