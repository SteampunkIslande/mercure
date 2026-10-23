use super::GitCheckError;

use crate::models::FileIdentification;

use super::DirContentsItem;
use log::warn;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DirContentsResponse {
    pub dir_contents: Vec<DirContentsItem>,
}

#[derive(Debug, Deserialize)]
struct FileContentsResponse {
    pub file_contents: DirContentsItem,
}

pub async fn list_yaml_forms(
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
            warn!(
                "Failed to list contents at {} (status {})",
                url,
                resp.status()
            );
            continue;
        }

        // Idiomatic let-else pattern replaces the verbose match block
        let Ok(body) = resp.json::<DirContentsResponse>().await else {
            warn!("Failed to parse directory contents at {}", url);
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

async fn is_file(
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
            return Err(anyhow::format_err!("Invalid form identification").into());
        }
    };
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(
            anyhow::format_err!("Failed to check if file exists: {}", resp.status()).into(),
        );
    }
    Ok(resp
        .json::<FileContentsResponse>()
        .await?
        .file_contents
        .item_type
        .as_str()
        == "file")
}

pub async fn git_get_file(
    base_url: &str,
    owner: &str,
    repo: &str,
    form_identification: &FileIdentification,
) -> Result<String, GitCheckError> {
    let client = Client::new();

    if !is_file(base_url, owner, repo, form_identification).await? {
        return Err(GitCheckError::NotAFile(format!(
            "{}",
            form_identification.path().display(),
        )));
    }

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
            return Err(anyhow::format_err!("Invalid form identification").into());
        }
    };

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow::format_err!("Failed to fetch file: {}", resp.status()).into());
    }
    Ok(resp.text().await?)
}
