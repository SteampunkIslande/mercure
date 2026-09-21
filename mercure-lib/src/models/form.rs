use std::{collections::HashSet, path::Path};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    config::{GitWebConfig, MercureConfig},
    models::{ModelError, groups},
    pipeline_exec::{GitCheckError, versionning},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
    pub source: Option<String>,
    pub userdefined: Option<bool>,
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Form {
    pub name: String,
    pub description: Option<String>,
    pub variables: Vec<Variable>,
    pub exec_type: Option<String>,
    pub trigger: Option<String>,
    pub workdir: Option<String>,
    pub exec: Option<String>,
    pub branch: Option<String>,
    pub file_path: Option<String>,
    pub groups: Option<Vec<String>>,
}

impl Form {
    pub async fn get_all_form_defs(config: &MercureConfig) -> Result<Vec<Form>, ModelError> {
        let all_branches = versionning::get_all_branches().await?;
        let MercureConfig { pipelines, .. } = config;

        let mut all_forms: Vec<Form> = Vec::new();

        match pipelines {
            GitWebConfig::GiteaV1 {
                base_url,
                owner,
                repo,
            } => {
                for branch_name in all_branches.iter() {
                    // 1. List yaml files on this branch
                    let items =
                        versionning::list_yaml_forms_giteav1(base_url, owner, repo, branch_name)
                            .await?;
                    // 2. Deserialize each form
                    for dir_item in items {
                        let mut form: Form = yaml_serde::from_str(
                            &versionning::get_file_giteav1(
                                base_url,
                                owner,
                                repo,
                                branch_name,
                                Path::new(&dir_item.path),
                            )
                            .await?,
                        )?;
                        form.branch = Some(branch_name.clone());
                        form.file_path = Some(dir_item.path);
                        all_forms.push(form);
                    }
                }
            }
            GitWebConfig::GitlabV4 { .. } => {
                // keep the original intent, but return a proper error instead of `todo!`
                return Err(GitCheckError::Unsupported(
                    "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
                )
                .into());
            }
        }

        Ok(all_forms)
    }

    /// Returns a tuple of (forms with an assigned group, forms with no assigned group).
    /// Forms with no assigned group are available to admin users only
    ///
    /// # Arguments
    ///
    /// - `pool` (`&SqlitePool`) - Database connection handle (not sure this is useful)
    /// - `group_id` (`i64`) - Group ID to retrieve available forms to
    ///
    /// # Returns
    ///
    /// - `Result<(Vec<Form>,Vec<Form>),ModelError>` - (forms with an assigned group, forms with no assigned group) as a result.
    ///
    /// # Errors
    ///
    /// Returns an error if...
    pub async fn get_form_list_items_for_group(
        pool: &SqlitePool,
        config: &MercureConfig,
        group_id: i64,
    ) -> Result<(Vec<Form>, Vec<Form>), ModelError> {
        let group_name = groups::Group::group_name_from_id(pool, group_id).await?;

        let forms_with_groups = Self::get_all_form_defs(config)
            .await?
            .into_iter()
            .filter(|f| {
                f.groups
                    .as_ref()
                    .map(|grps| grps.contains(&group_name))
                    .unwrap_or(false)
            })
            .collect();

        let forms_without_groups = Self::get_all_form_defs(config)
            .await?
            .into_iter()
            .filter(|f| {
                f.groups
                    .as_ref()
                    .map(|grps| grps.contains(&group_name))
                    .unwrap_or(false)
            })
            .collect();

        Ok((forms_with_groups, forms_without_groups))
    }

    pub async fn get_form(branch: &str, form_path: &Path) -> Result<Form, ModelError> {
        let MercureConfig { pipelines, .. } = crate::config::get_mercure_config();

        match &pipelines {
            GitWebConfig::GiteaV1 {
                base_url,
                owner,
                repo,
            } => {
                let content =
                    versionning::get_file_giteav1(base_url, owner, repo, branch, form_path).await?;
                let mut form: Form = yaml_serde::from_str(&content)?;
                form.branch = Some(branch.to_string());
                form.file_path = Some(form_path.display().to_string());
                Ok(form)
            }
            GitWebConfig::GitlabV4 { .. } => Err(GitCheckError::Unsupported(
                "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
            )
            .into()),
        }
    }

    /// Returns all the groups this form is part of (according to its definition of groups: Vec<String>)
    ///
    /// # Arguments
    ///
    /// - `pool` (`&SqlitePool`) - A handle to the database
    ///
    /// # Returns
    ///
    /// - `Result<Vec<i64>, ModelError>` - On success, the list of group ids that are associated with this form
    ///
    /// # Errors
    ///
    /// Returns an error in case we cannot retrieve the list of groups from the database
    pub async fn get_group_ids(&self, pool: &SqlitePool) -> Result<Vec<i64>, ModelError> {
        let self_groups: HashSet<&str> = self
            .groups
            .as_ref()
            .map(|groups| groups.iter().map(String::as_str).collect())
            .unwrap_or(HashSet::new());

        let groups = groups::Group::get_groups_with_ids(pool).await?;

        let ids = groups
            .into_iter()
            .filter(|g| self_groups.contains(g.name.as_str()))
            .map(|g| g.id)
            .collect();

        Ok(ids)
    }
}
