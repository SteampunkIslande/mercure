use super::GitCheckError;

use crate::models::FileIdentification;

use super::DirContentsItem;
use log::warn;
use reqwest::Client;
use serde::Deserialize;

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
    use std::time::Duration;
    use tokio::process::Command;
    use walkdir::WalkDir;

    let tmp_id = format!(
        "smaug_repo_{}_{}_{}",
        owner,
        repo,
        branch_name.replace("/", "_")
    );
    let tmp_root = std::env::temp_dir().join(tmp_id);
    let _ = tokio::fs::create_dir_all(&tmp_root)
        .await
        .map_err(|e| GitCheckError::CommandFailed(format!("tmp dir create failed: {}", e)))?;

    let repo_path = tmp_root.join(repo);
    let clone_url = format!("{base_url}/{owner}/{repo}.git");

    let mut clone_cmd = Command::new("git");
    clone_cmd
        .arg("clone")
        .arg("--branch")
        .arg(branch_name)
        .arg(&clone_url)
        .arg(&repo_path);

    let mut clone_child = clone_cmd
        .spawn()
        .map_err(|e| GitCheckError::CommandFailed(format!("git spawn failed: {}", e)))?;
    let clone_res = tokio::time::timeout(Duration::from_secs(60), clone_child.wait()).await;
    let clone_ok = match clone_res {
        Ok(Ok(status)) => status.success(),
        _ => {
            let _ = tokio::fs::remove_dir_all(&tmp_root).await;
            return Err(GitCheckError::CommandFailed(
                "git clone failed or timed out".into(),
            ));
        }
    };

    if !clone_ok {
        if let Err(e) = tokio::fs::remove_dir_all(&tmp_root).await {
            warn!(
                "Cannot delete temp dir '{}', error {}",
                tmp_root.display(),
                e
            )
        }
        return Err(GitCheckError::CommandFailed("git clone failed".into()));
    }

    let mut results = Vec::new();
    for entry in WalkDir::new(&repo_path) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy();
        if !(file_name.ends_with(".yml") || file_name.ends_with(".yaml")) {
            continue;
        }
        let full_path = entry.path();
        let rel_path = match full_path.strip_prefix(&repo_path) {
            Ok(p) => p.to_path_buf(),
            Err(_) => continue,
        };
        let has_form = rel_path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy().to_ascii_lowercase();
            s == ".form" || s == ".forms"
        });
        if !has_form {
            continue;
        }
        results.push(DirContentsItem {
            name: file_name.into_owned(),
            path: rel_path,
            item_type: "file".to_string(),
            content: tokio::fs::read_to_string(full_path).await?,
        });
    }

    if let Err(e) = tokio::fs::remove_dir_all(&tmp_root).await {
        warn!(
            "Couldn't remove temp dir {} (error: {})",
            tmp_root.display(),
            e
        )
    }
    Ok(results)
}

async fn is_file(
    base_url: &str,
    owner: &str,
    repo: &str,
    file_identification: &FileIdentification,
) -> Result<bool, GitCheckError> {
    let client = Client::new();
    let (file_path, reference_) = match file_identification {
        FileIdentification::GitBranchAndPath { branch, file_path } => (file_path, branch),
        FileIdentification::GitCommit {
            commit_hash,
            file_path,
        } => (file_path, commit_hash),
        FileIdentification::None => {
            return Err(anyhow::format_err!("Invalid").into());
        }
    };
    let url = format!(
        "{}/api/v1/repos/{}/{}/contents-ext/{}?ref={}",
        base_url,
        owner,
        repo,
        file_path.display(),
        reference_
    );
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
    file_identification: &FileIdentification,
) -> Result<String, GitCheckError> {
    let client = Client::new();

    if !is_file(base_url, owner, repo, file_identification).await? {
        return Err(GitCheckError::NotAFile(format!(
            "{}",
            file_identification.path().display(),
        )));
    }

    let url = match file_identification {
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
