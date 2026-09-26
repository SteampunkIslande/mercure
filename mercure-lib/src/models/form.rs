use std::{collections::HashSet, path::PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    config::{GitWebConfig, MercureConfig},
    models::{ModelError, groups},
    pipeline_exec::{GitCheckError, versionning},
};

#[derive(thiserror::Error, Debug)]
pub enum FormDefinitionError {
    #[error(transparent)]
    SerdeError(#[from] yaml_serde::Error),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(tag = "type")]
pub enum VariableType {
    FromURL {
        source: String,
    },
    ValuesList {
        values: Vec<String>,
    },
    /// With this type, the user is prompted a date and the date is returned
    DateEdit,
    /// By default, a variable is set by the user from a text field
    #[default]
    LineEdit,
    /// The user will have to upload a file to the server
    ExistingFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "exec_type")]
pub enum FormExecType {
    #[serde(rename = "shell")]
    Shell { exec: String },
    #[serde(rename = "script")]
    Script { script_name: PathBuf },
}

impl Default for FormExecType {
    fn default() -> Self {
        Self::Shell {
            exec: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Variable {
    pub name: String,
    pub title: String,
    pub description: String,
    #[serde(flatten)]
    pub variable_type: VariableType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Form {
    pub name: String,
    pub description: String,
    pub variables: Vec<Variable>,
    #[serde(flatten)]
    pub exec_type: FormExecType,
    pub workdir: String,
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
                                &dir_item.path,
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

    pub async fn get_form(branch: &str, form_path: &str) -> Result<Form, ModelError> {
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
                form.file_path = Some(form_path.to_string());
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

    pub async fn check_validity(&self) -> Result<(), FormDefinitionError> {
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use crate::models::VariableType::ValuesList;

    use super::*;

    #[test]
    fn test_form_serialize() {
        let expected = Form {
                        name: "Smaug-v3 simple".to_string(),
                        description: "Démultiplexe à partir d'un dossier de run brut, \
                        lance l'analyse smaug avec le BED spécifié, puis copie \
                        le résultat sur le NAS."
                            .to_string(),
                        variables: vec![
                            Variable {
                                name: "bed".to_string(),
                                title: "BED".to_string(),
                                description: "Le fichier BED à utiliser pour cette analyse".to_string(),
                                variable_type: VariableType::FromURL {
                                    source: "/mercure/api/aux/list_beds".to_string()
                                },
                                
                            },
                            Variable {
                                name: "genome".to_string(),
                                title: "Génome".to_string(),
                                
                                description: "Le génome à utiliser".to_string(),
                                variable_type: ValuesList {
                                    values: vec!["hg19".to_string(), "hg38".to_string()]
                                }
                            },
                            Variable {
                                name: "indir".to_string(),
                                title: "Dossier BCL".to_string(),
                                description: "Le dossier de run brut Illumina".to_string(),
                                variable_type: VariableType::FromURL {
                                    source: "/mercure/api/aux/list_illumina_dirs".to_string()
                                },
                            },
                            Variable {
                                name: "outdir".to_string(),
                                title: "Dossier de sortie".to_string(),
                                description: "Le dossier de travail pour snakemake".to_string(),
                                variable_type: VariableType::LineEdit,
                            },
                            Variable {
                                name: "panel".to_string(),
                                title: "Nom du panel".to_string(),
                                description: "Le nom du panel".to_string(),
                                variable_type: VariableType::FromURL {
                                    source: "/mercure/api/aux/list_panels".to_string()
                                },
                            },
                            Variable {
                                name: "sample_sheet".to_string(),
                                title: "SampleSheet".to_string(),
                                description: "La samplesheet, espèce de neuneu".to_string(),
                                variable_type: VariableType::ExistingFile,
                            }
                        ],
                        branch:None,
                        file_path: None,

                        workdir: "outdir".to_string(),
                        groups: Some(vec!["Admin".to_string(),"Génétique".to_string()]),
                        exec_type: FormExecType::Shell { exec: r#"# Copie de la samplesheet dans le bon dossier
rsync {{ sample_sheet }} {{ indir }}/SampleSheet.csv

# Démultiplexage
snakemake -s pipelines/demul/Snakefile --config indir={{ indir }} outdir={{ outdir }}

# Copie vers MOABI 
rsync -a --info=progress2 "{{ outdir }}/fastq" user@depot:/where/it/should/go

# Smaug v3
snakemake -s pipelines/smaug-v3/Snakefile --configfile pipelines/smaug-v3/config.yaml --config indir={{ outdir }} outdir={{ outdir }}/tentative-{{ attempt_id }}

# Copie NAS
rsync -a --info=progress2 "{{ outdir }}/{bam,vcf,reports}" /mnt/nas/analysis/{{ panel }}
"#.to_string() }};
        let form: Form = yaml_serde::from_str(include_str!("../../../.forms/smaug-basique.yaml"))
            .expect("Cannot serialize");

        let expected_str = serde_json::to_string(&expected).expect("Cannot serialize");
        let form_str = serde_json::to_string(&form).expect("Cannot serialize back");

        assert_eq!(
            expected_str, form_str,
            "Left should be:\n-----\n {} and right should be:\n------\n {}",
            expected_str, form_str
        );
    }
}
