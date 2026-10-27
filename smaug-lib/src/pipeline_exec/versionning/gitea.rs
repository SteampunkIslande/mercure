use super::{DirContentsItem, GitCheckError};

use crate::models::FileIdentification;

use anyhow::Context;
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
    tokio::fs::create_dir_all(&tmp_root)
        .await
        .with_context(|| {
            format!(
                "Impossible de créer le dossier temporaire {}",
                tmp_root.display()
            )
        })?;

    let repo_path = tmp_root.join(repo);
    let clone_url = format!("{base_url}/{owner}/{repo}.git");

    let mut clone_cmd = Command::new("git");
    clone_cmd
        .arg("clone")
        .arg("--branch")
        .arg(branch_name)
        .arg(&clone_url)
        .arg(&repo_path);

    let mut clone_child = clone_cmd.spawn().with_context(|| {
        format!("Impossible de lancer `git clone {clone_url}` (branche {branch_name})")
    })?;
    let clone_res = tokio::time::timeout(Duration::from_secs(60), clone_child.wait()).await;
    let clone_ok = match clone_res {
        Ok(Ok(status)) => status.success(),
        Ok(Err(e)) => {
            let _ = tokio::fs::remove_dir_all(&tmp_root).await;
            return Err(e).with_context(|| {
                format!("Erreur en attendant la fin de `git clone` de la branche {branch_name}")
            })?;
        }
        Err(_) => {
            let _ = tokio::fs::remove_dir_all(&tmp_root).await;
            return Err(GitCheckError::CommandFailed(
                "Échec ou expiration du clone git".into(),
            ));
        }
    };

    if !clone_ok {
        if let Err(e) = tokio::fs::remove_dir_all(&tmp_root).await {
            log::warn!(
                "Cannot delete temp dir '{}', error {}",
                tmp_root.display(),
                e
            )
        }
        return Err(GitCheckError::CommandFailed("Échec du clone git".into()));
    }

    // Per-entry failures (unreadable directory, unrelated path) are logged and
    // skipped: one bad entry must not remove every other form from the cache.
    let mut results = Vec::new();
    for entry in WalkDir::new(&repo_path) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                log::warn!(
                    "Impossible de parcourir une entrée de {}: {}",
                    repo_path.display(),
                    e
                );
                continue;
            }
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
            Ok(rel_path) => rel_path.to_path_buf(),
            Err(e) => {
                log::warn!(
                    "Impossible de calculer le chemin relatif de {}: {}",
                    full_path.display(),
                    e
                );
                continue;
            }
        };
        let has_form = rel_path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy().to_ascii_lowercase();
            s == ".form" || s == ".forms"
        });
        if !has_form {
            continue;
        }
        let content = tokio::fs::read_to_string(full_path)
            .await
            .with_context(|| format!("Impossible de lire le fichier {}", full_path.display()))?;
        results.push(DirContentsItem {
            name: file_name.into_owned(),
            path: rel_path,
            item_type: "file".to_string(),
            content,
        });
    }

    if let Err(e) = tokio::fs::remove_dir_all(&tmp_root).await {
        log::warn!(
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
            return Err(GitCheckError::NotAFile(
                "Un formulaire doit être identifié par une branche ou un commit".to_string(),
            ));
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
    let resp = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Impossible d'interroger l'API Gitea ({url})"))?;
    if !resp.status().is_success() {
        return Err(GitCheckError::CommandFailed(format!(
            "Impossible de vérifier si le fichier existe : {}",
            resp.status()
        )));
    }
    Ok(resp
        .json::<FileContentsResponse>()
        .await
        .with_context(|| format!("Réponse inattendue de l'API Gitea pour {url}"))?
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
            return Err(GitCheckError::NotAFile(
                "Un formulaire doit être identifié par une branche ou un commit".to_string(),
            ));
        }
    };

    let resp = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("Impossible de récupérer le fichier {url}"))?;
    if !resp.status().is_success() {
        return Err(GitCheckError::CommandFailed(format!(
            "Impossible de récupérer le fichier : {}",
            resp.status()
        )));
    }
    Ok(resp
        .text()
        .await
        .with_context(|| format!("Impossible de lire le contenu de {url}"))?)
}
