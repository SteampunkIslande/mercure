use std::path::PathBuf;

use anyhow::Context;
use log::warn;
use minijinja::Environment;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
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
    #[error("La variable `{}` apparaît en plusieurs exemplaires",.0)]
    DuplicateVarError(String),
    #[error("Erreur de formulaire: {0}")]
    InvalidData(String),
    #[error(transparent)]
    MinijinjaError(#[from] minijinja::Error),
    #[error(transparent)]
    AnyHowError(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(tag = "type")]
pub enum VariableType {
    ListFromURL {
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
    /// The user will have to upload a file to the server. It is the html's responsibility to post a valid, server-side file name as the variable value
    ExistingFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Variable {
    pub name: String,
    pub title: String,
    pub description: String,
    #[serde(flatten)]
    pub variable_type: VariableType,
}

impl Variable {
    pub fn to_html_safe(&self) -> String {
        let Variable {
            name,
            title,
            description,
            variable_type,
        } = self;

        let description = description.replace('\n', "<br>");

        let widget: String = match variable_type {
            VariableType::DateEdit => {
                format!(r#"<input type="date" id="{name}" name="{name}">"#)
            }
            VariableType::ExistingFile => {
                format!(r#"<input type="file" id="{name}" name="{name}">"#)
            }
            VariableType::ListFromURL { source } => {
                // Render an empty select with a data-source attribute.
                // The JS reads `dataset.source`, fetch the URL, and populate the <option>s.
                format!(r#"<select id="{name}" name="{name}" data-source="{source}"></select>"#)
            }
            VariableType::ValuesList { values } => {
                let options = values
                    .iter()
                    .map(|v| format!(r#"<option value="{v}">{v}</option>"#))
                    .collect::<Vec<String>>()
                    .join("\n");

                format!(
                    r#"<select id="{name}" name="{name}">
                    {options}
                    </select>"#
                )
            }
            VariableType::LineEdit => {
                format!(r#"<input type="text" id="{name}" name="{name}">"#)
            }
        };

        format!(
            r#"<div class="card-container" id="{name}-card">
            <label>{title}</label>
            <span>{description}</span>
            {widget}
        </div>"#
        )
    }
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
                // For each branch
                for branch_name in all_branches.iter() {
                    // 1. List yaml files on this branch
                    let items =
                        versionning::list_yaml_forms_giteav1(base_url, owner, repo, branch_name)
                            .await
                            .unwrap_or_default();
                    // For each form in given branch
                    for dir_item in items {
                        // fetch file
                        let content = match versionning::get_file_giteav1(
                            base_url,
                            owner,
                            repo,
                            branch_name,
                            &dir_item.path,
                        )
                        .await
                        {
                            Ok(c) => c,
                            Err(_) => continue,
                        };

                        // parse yaml
                        let mut form: Form = match yaml_serde::from_str(&content) {
                            Ok(f) => f,
                            Err(e) => {
                                warn!(
                                    "Impossible de sérialiser {} sur la branche {}: {}",
                                    dir_item.path, branch_name, e
                                );
                                continue;
                            }
                        };

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
                form.check_validity(&json!({})).await?;
                Ok(form)
            }
            GitWebConfig::GitlabV4 { .. } => Err(GitCheckError::Unsupported(
                "L'API Gitlab V4 n'est pas encore prise en charge!".into(),
            )
            .into()),
        }
    }

    pub async fn check_validity(
        &self,
        context: &impl Serialize,
    ) -> Result<(), FormDefinitionError> {
        self.try_render_template(context)?;
        Ok(())
    }

    pub fn try_render_template(&self, context: &impl Serialize) -> Result<(), FormDefinitionError> {
        let template = match &self.exec_type {
            FormExecType::Script { script_name } => {
                std::fs::read_to_string(&script_name).context("Impossible de lire le template")?
            }
            FormExecType::Shell { exec } => exec.to_string(),
        };
        let env = Environment::new();
        env.render_str(&template, context)
            .context("Erreur jinja (template incorrect)")?;
        Ok(())
    }
}

/// Aperçu d'un formulaire, mis en cache en base de données.
///
/// Un formulaire est défini de façon source de vérité par sa paire
/// (`branch`, `file_path`) dans le dépôt git : cette structure ne retient
/// exactement que cette paire, avec un nom et une description, mais laisse la
/// logique (définition des variables, du mode d'exécution) au fichier YAML de définition.
///
/// La visibilité d'un `FormDef` pour un groupe est portée par la table
/// `FormDefHasGroup` (jointure simple en SQL). Cette table est reconstruite
/// par `FormDef::refresh_cache`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachedForm {
    /// L'ID d'un formulaire. **Ne pas** l'utiliser pour identifier un formulaire en tant que tel.
    pub form_def_id: i64,
    pub branch: String,
    pub file_path: String,
    pub name: String,
    pub description: String,
}

impl CachedForm {
    /// Rebuild the FormDef cache from the actual Form repository.
    ///
    /// This is an all-in-one transaction: it deletes every stale cached FormDef,
    /// then re-inserts all the forms found on the git repository
    /// (see `Form::get_all_form_defs`), and rebuilds `FormDefHasGroup` from the
    /// `groups` field of each form definition.
    ///
    /// # Arguments
    ///
    /// - `pool` (`&SqlitePool`) - A handle to the database
    /// - `config` (`&MercureConfig`) - Application configuration (git repository, mainly)
    ///
    /// # Returns
    ///
    /// - `Result<Vec<FormDef>, ModelError>` - On success, the whole cache (which is equal to
    ///   the `FormDef`s found in the repository)
    ///
    /// # Errors
    ///
    /// Returns an error in case of database or git repository issues.
    pub async fn refresh_cache(
        pool: &SqlitePool,
        config: &MercureConfig,
    ) -> Result<Vec<Self>, ModelError> {
        let all_forms = Form::get_all_form_defs(config).await?;
        Self::rebuild_cache_from_forms(pool, all_forms).await
    }

    /// Synchronously rebuilds `FormDefs` and `FormDefHasGroup` from an
    /// already-scanned list of form definitions (see `Form::get_all_form_defs`).
    pub(crate) async fn rebuild_cache_from_forms(
        pool: &SqlitePool,
        all_forms: Vec<Form>,
    ) -> Result<Vec<Self>, ModelError> {
        let all_groups = groups::Group::get_groups_with_ids(pool).await?;

        let mut tx = pool.begin().await?;

        sqlx::query("DELETE FROM FormDefHasGroup")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM FormDefs")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE from sqlite_sequence where name='FormDefs'")
            .execute(&mut *tx)
            .await?;

        let mut cached = Vec::new();
        for form in all_forms {
            let Form {
                branch: Some(branch),
                name,
                description,
                file_path: Some(file_path),
                groups,
                ..
            } = form
            else {
                continue;
            };

            let form_def_id: i64 = sqlx::query(
                r#"
                INSERT INTO FormDefs (branch, file_path, name, description) VALUES (?, ?, ?, ?)
                RETURNING form_def_id
                "#,
            )
            .bind(&branch)
            .bind(&file_path)
            .bind(&name)
            .bind(&description)
            .fetch_one(&mut *tx)
            .await?
            .try_get("form_def_id")?;

            if let Some(groups) = groups {
                for group_name in groups {
                    match all_groups.iter().find(|g| g.name == group_name) {
                        Some(group) => {
                            sqlx::query(
                                r#"INSERT INTO FormDefHasGroup (form_def_id, group_id) VALUES (?, ?)"#,
                            )
                            .bind(form_def_id)
                            .bind(group.id)
                            .execute(&mut *tx)
                            .await?;
                        }
                        None => log::warn!(
                            "Le formulaire {branch}/{file_path} attend le groupe {group_name}, qui n'existe pas en base"
                        ),
                    }
                }
            }

            cached.push(CachedForm {
                form_def_id,
                branch,
                file_path,
                description,
                name,
            });
        }

        tx.commit().await?;

        Ok(cached)
    }

    /// All the cached `FormDef`s (from the database only, no HTTP request)
    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Self>, ModelError> {
        Ok(sqlx::query(
            r#"
            SELECT form_def_id, branch, file_path, name, description FROM FormDefs ORDER BY branch, file_path
            "#,
        )
        .fetch_all(pool)
        .await?
        .iter()
        .filter_map(form_def_from_row)
        .collect())
    }

    /// Lists (branch, file_path) pairs that are visible to given groups.
    ///
    /// Until now, that info was scattered in form definitions; it now lives in the
    /// `FormDefs` and `FormDefHasGroup` tables, and is resolved with a simple SQL query.
    ///
    /// # Arguments
    ///
    /// - `pool` (`&SqlitePool`) - A handle to the database
    /// - `groups` (`Vec<Group>`) - The groups doing the search
    ///
    /// # Returns
    ///
    /// - `Result<Vec<(String, String)>, ModelError>` - The list of visible (branch, file_path) pairs
    ///
    /// # Errors
    ///
    /// Returns an error in case of database issues.
    pub async fn visible_form_pairs(
        pool: &SqlitePool,
        groups: Vec<groups::Group>,
    ) -> Result<Vec<(String, String)>, ModelError> {
        let group_ids: Vec<i64> = groups.into_iter().map(|g| g.id).collect();

        if group_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = vec!["?"; group_ids.len()].join(", ");

        let query_str = format!(
            r#"
            SELECT DISTINCT fd.branch, fd.file_path
            FROM FormDefs fd
            JOIN FormDefHasGroup fhg ON fhg.form_def_id = fd.form_def_id
            WHERE fhg.group_id IN ({placeholders})
            ORDER BY fd.branch, fd.file_path
            "#
        );

        let mut query = sqlx::query(&query_str);

        for group_id in group_ids {
            query = query.bind(group_id);
        }

        Ok(query
            .fetch_all(pool)
            .await?
            .into_iter()
            .filter_map(|row| Some((row.try_get("branch").ok()?, row.try_get("file_path").ok()?)))
            .collect())
    }

    /// The group ids that a (branch, file_path) pair is associated with.
    ///
    /// This is a simple SQL query, and does not require any HTTP request to the
    /// git repository: the association lives in the `FormDefHasGroup` table. A
    /// form that is not cached (or has no group) returns an empty list; it is
    /// then considered admin-only.
    ///
    /// # Arguments
    ///
    /// - `pool` (`&SqlitePool`) - A handle to the database
    /// - `branch` (`&str`) - The branch the form lives on
    /// - `form_path` (`&str`) - The path to the form yaml file, within said branch
    ///
    /// # Returns
    ///
    /// - `Result<Vec<i64>, ModelError>` - On success, the list of group ids that are associated with this form
    ///
    /// # Errors
    ///
    /// Returns an error in case of database issues.
    pub async fn get_group_ids(
        pool: &SqlitePool,
        branch: &str,
        form_path: &str,
    ) -> Result<Vec<i64>, ModelError> {
        Ok(sqlx::query(
            r#"
            SELECT fhg.group_id
            FROM FormDefs fd
            JOIN FormDefHasGroup fhg ON fhg.form_def_id = fd.form_def_id
            WHERE fd.branch = ? AND fd.file_path = ?
            "#,
        )
        .bind(branch)
        .bind(form_path)
        .fetch_all(pool)
        .await?
        .into_iter()
        .filter_map(|row| row.try_get("group_id").ok())
        .collect())
    }

    /// Returns a tuple of (cached form defs with an assigned group, cached form defs
    /// with no assigned group).
    /// Form defs with no assigned group are available to admin users only.
    ///
    /// Both lists come straight from the cache, with a simple SQL query each.
    ///
    /// # Arguments
    ///
    /// - `pool` (`&SqlitePool`) - A handle to the database
    /// - `group_id` (`i64`) - Group ID to retrieve available forms to
    ///
    /// # Returns
    ///
    /// - `Result<(Vec<FormDef>, Vec<FormDef>), ModelError>` - (form defs with an assigned group,
    ///   form defs with no assigned group) as a result.
    ///
    /// # Errors
    ///
    /// Returns an error in case of database issues.
    pub async fn get_form_defs_for_group(
        pool: &SqlitePool,
        group_id: Option<i64>,
    ) -> Result<Vec<CachedForm>, ModelError> {
        let forms = match group_id {
            Some(group_id) => {
                sqlx::query(
                    r#"
                SELECT fd.form_def_id, fd.branch, fd.file_path, name, description
                FROM FormDefs fd
                JOIN FormDefHasGroup fhg ON fhg.form_def_id = fd.form_def_id
                WHERE fhg.group_id = ?
                ORDER BY fd.branch, fd.file_path
                "#,
                )
                .bind(group_id)
                .fetch_all(pool)
                .await?
            }
            None => {
                sqlx::query(
                    r#"
                SELECT fd.form_def_id, fd.branch, fd.file_path, name, description
FROM FormDefs fd
WHERE NOT EXISTS (
    SELECT 1 
    FROM FormDefHasGroup fhg 
    WHERE fhg.form_def_id = fd.form_def_id
)
ORDER BY fd.branch, fd.file_path"#,
                )
                .fetch_all(pool)
                .await?
            }
        };

        Ok(forms.iter().filter_map(form_def_from_row).collect())
    }
}

/// Builds a `FormDef` from a row selecting `form_def_id`, `branch` and `file_path`
fn form_def_from_row(row: &sqlx::sqlite::SqliteRow) -> Option<CachedForm> {
    Some(CachedForm {
        form_def_id: row.try_get("form_def_id").ok()?,
        branch: row.try_get("branch").ok()?,
        file_path: row.try_get("file_path").ok()?,
        description: row.try_get("description").ok()?,
        name: row.try_get("name").ok()?,
    })
}

#[cfg(test)]
mod test {

    use std::fmt::{Debug, Display};

    use crate::models::VariableType::{DateEdit, ExistingFile, ValuesList};

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
                                variable_type: VariableType::ListFromURL {
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
                                variable_type: VariableType::ListFromURL {
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
                                variable_type: VariableType::ListFromURL {
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

    /// Just a helper to print what the assert_eq actually got on the left side.
    /// Applies to types that implement the Display trait.
    #[track_caller]
    fn assert_eq_display_helper<T, U>(left: &T, right: &U)
    where
        T: Display + Debug + PartialEq<U> + ?Sized,
        U: Debug + ?Sized,
    {
        assert_eq!(
            left, right,
            "\n------BEGIN_LEFT------\n{}\n-------END_LEFT-------\n",
            left
        );
    }

    #[test]
    fn test_vars_to_html_values_list() {
        // Test d'une variable de type ValuesList
        let v = Variable {
            title: "Une variable".to_string(),
            name: "v1".to_string(),
            description: "Une simple description".to_string(),
            variable_type: ValuesList {
                values: vec!["A".to_string(), "B".to_string()],
            },
        };
        let v_html = v.to_html_safe();
        assert_eq_display_helper(
            &v_html,
            r#"<div class="card-container" id="v1-card">
            <label>Une variable</label>
            <span>Une simple description</span>
            <select id="v1" name="v1">
                    <option value="A">A</option>
<option value="B">B</option>
                    </select>
        </div>"#,
        );
    }

    #[test]
    fn test_vars_to_html_existing_file() {
        //
        let v = Variable {
            title: "Une autre variable".to_string(),
            name: "v2".to_string(),
            description: "Whatever".to_string(),
            variable_type: ExistingFile,
        };
        let v_html = v.to_html_safe();
        assert_eq_display_helper(
            &v_html,
            r#"<div class="card-container" id="v2-card">
            <label>Une autre variable</label>
            <span>Whatever</span>
            <input type="file" id="v2" name="v2">
        </div>"#,
        );
    }

    #[test]
    fn test_vars_to_html_date_edit() {
        // Test d'une variable de type ValuesList
        let v1 = Variable {
            title: "Une variable".to_string(),
            name: "v1".to_string(),
            description: "Une simple description".to_string(),
            variable_type: DateEdit,
        };
        let v1_html = v1.to_html_safe();
        assert_eq_display_helper(
            &v1_html,
            r#"<div class="card-container" id="v1-card">
            <label>Une variable</label>
            <span>Une simple description</span>
            <input type="date" id="v1" name="v1">
        </div>"#,
        );
    }
}

/// Tests du cache SQL (`FormDefs` / `FormDefHasGroup`)
#[cfg(test)]
mod form_def_cache_tests {
    use sqlx::SqlitePool;

    use crate::models::{CachedForm, Form, Group};

    async fn test_pool() -> SqlitePool {
        // Le cache en mémoire vit par connexion : une seule connexion dans le pool
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("pool en mémoire");
        crate::db::run_migrations(&pool).await.expect("migrations");
        pool
    }

    fn form(name: &str, branch: &str, path: &str, groups: &[&str]) -> Form {
        Form {
            name: name.to_string(),
            description: String::new(),
            variables: vec![],
            exec_type: Default::default(),
            workdir: String::new(),
            branch: Some(branch.to_string()),
            file_path: Some(path.to_string()),
            groups: Some(groups.iter().map(|s| s.to_string()).collect()),
        }
    }

    fn group(id: i64) -> Group {
        Group {
            id,
            name: format!("g{id}"),
        }
    }

    #[tokio::test]
    async fn test_cache_lists_and_visibility() {
        let pool = test_pool().await;

        let bio = Group::add_group(&pool, "Bio").await.expect("add Bio");
        let chimie = Group::add_group(&pool, "Chimie").await.expect("add Chimie");

        let cached = CachedForm::rebuild_cache_from_forms(
            &pool,
            vec![
                form("F1", "main", ".forms/f1.yaml", &["Bio"]),
                form("F2", "dev", ".forms/f2.yaml", &[]),
                // Groupe inexistant en base : ignoré
                form("F3", "main", ".forms/f3.yaml", &["Fantome"]),
            ],
        )
        .await
        .expect("rebuild cache");

        assert_eq!(cached.len(), 3);

        let all = CachedForm::get_all(&pool).await.expect("get_all");
        assert_eq!(
            all,
            vec![
                CachedForm {
                    form_def_id: all[0].form_def_id,
                    branch: "dev".into(),
                    file_path: ".forms/f2.yaml".into(),
                    description: "".to_string(),
                    name: "F2".to_string()
                },
                CachedForm {
                    form_def_id: all[1].form_def_id,
                    branch: "main".into(),
                    file_path: ".forms/f1.yaml".into(),
                    description: "".to_string(),
                    name: "F1".to_string()
                },
                CachedForm {
                    form_def_id: all[2].form_def_id,
                    branch: "main".into(),
                    file_path: ".forms/f3.yaml".into(),
                    description: "".to_string(),
                    name: "F3".to_string()
                },
            ]
        );

        let forms_with_group = CachedForm::get_form_defs_for_group(&pool, Some(bio))
            .await
            .expect("for group");
        assert_eq!(forms_with_group.len(), 1);
        assert_eq!(
            (
                forms_with_group[0].branch.as_str(),
                forms_with_group[0].file_path.as_str()
            ),
            ("main", ".forms/f1.yaml")
        );

        let forms_without_group = CachedForm::get_form_defs_for_group(&pool, None)
            .await
            .expect("for no group");
        assert_eq!(forms_without_group.len(), 2);

        assert_eq!(
            CachedForm::get_group_ids(&pool, "main", ".forms/f1.yaml")
                .await
                .expect("ids"),
            vec![bio]
        );
        assert!(
            CachedForm::get_group_ids(&pool, "dev", ".forms/f2.yaml")
                .await
                .expect("ids")
                .is_empty()
        );
        assert!(
            CachedForm::get_group_ids(&pool, "main", ".forms/f3.yaml")
                .await
                .expect("ids")
                .is_empty()
        );

        assert_eq!(
            CachedForm::visible_form_pairs(&pool, vec![group(bio)])
                .await
                .expect("pairs"),
            vec![("main".to_string(), ".forms/f1.yaml".to_string())]
        );
        assert!(
            CachedForm::visible_form_pairs(&pool, vec![group(chimie)])
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            CachedForm::visible_form_pairs(&pool, vec![])
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn test_cache_refresh_purges_stale_entries() {
        let pool = test_pool().await;

        Group::add_group(&pool, "Bio").await.expect("add Bio");

        CachedForm::rebuild_cache_from_forms(
            &pool,
            vec![form("F1", "main", ".forms/f1.yaml", &["Bio"])],
        )
        .await
        .expect("first rebuild");

        // Un nouveau scan ne trouve plus que F1, sans groupe
        CachedForm::rebuild_cache_from_forms(
            &pool,
            vec![form("F1", "main", ".forms/f1.yaml", &[])],
        )
        .await
        .expect("second rebuild");

        let all = CachedForm::get_all(&pool).await.expect("get_all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].branch, "main");
        assert_eq!(all[0].file_path, ".forms/f1.yaml");

        // L'ancienne association (F1 -> Bio) ne doit plus exister
        assert!(
            CachedForm::get_group_ids(&pool, "main", ".forms/f1.yaml")
                .await
                .expect("ids")
                .is_empty()
        );

        let forms_with_group = CachedForm::get_form_defs_for_group(&pool, Some(1))
            .await
            .expect("for group");
        let forms_without_group = CachedForm::get_form_defs_for_group(&pool, None)
            .await
            .expect("for no group");
        assert!(forms_with_group.is_empty());
        assert_eq!(forms_without_group.len(), 1);
    }
}
