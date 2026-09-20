use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    config::{GitWebConfig, MercureConfig},
    models::ModelError,
    pipeline_exec::{GitCheckError, versionning},
};

#[derive(Deserialize, Serialize)]
pub struct Form {
    pub groups: Option<Vec<String>>,
    pub branch: String,
    pub file_name: String,
    pub file_path: String,
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
                        let form = yaml_serde::from_str(
                            &versionning::get_file_giteav1(
                                base_url,
                                owner,
                                repo,
                                branch_name,
                                Path::new(&dir_item.path),
                            )
                            .await?,
                        )?;
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
        group_id: i64,
    ) -> Result<(Vec<Form>, Vec<Form>), ModelError> {
        todo!()
    }

    pub async fn get_form(branch: &str, form_path: &Path) -> Result<Form, ModelError> {
        todo!()
    }
}
