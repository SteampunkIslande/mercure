use crate::launchers_check::get_current_revision;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;

use crate::config;
use crate::models::ModelError;

use super::groups::Group;

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum IndirType {
    #[default]
    BclDir,
    AnalysisDir,
    OntDir,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HgFormListItem {
    pub formid: i64,
    pub enabled: bool,
    pub form_name: String,
    pub version: i64,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum UserDefinedVar {
    FromValuesList {
        allowed: Vec<String>,
    },
    Constant(String),
    #[default]
    RunDefined,
    Invalid,
}

/// Struct used when submitting a new form definition
/// Only used when sending data from the browser to the server
/// See editform.html.jinja2 for more info
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HgFormDefSubmission {
    pub pipeline_name: String,
    pub launcher_name: String,
    pub form_name: String,
    pub enabled: bool,
    pub version: i32,
    pub groups: Vec<Group>,
    pub user_defined_vars: Option<HashMap<String, UserDefinedVar>>,
    pub indir_type: IndirType,
}

/// Struct used to define a form template
/// Only read from JSON, defined within the browser
/// See editform.html.jinja2 for more info
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct HgFormDef {
    pub form_id: i64,
    pub pipeline_name: String,
    pub launcher_name: String,
    pub form_name: String,
    pub enabled: bool,
    pub version: i32,
    pub groups: Vec<Group>,
    pub user_defined_vars: Option<HashMap<String, UserDefinedVar>>,
    pub indir_type: IndirType,
    pub latest_launcher_revision: Option<String>,
}

/// This type helps admin users define a form
///
/// Tables:
/// Formdef: form_id,pipeline_name,launcher_name,form_name
/// FormdefHasGroup: form_id,group_id
/// Groups: group_id,group_name
/// GroupHasUser: group_id,user_id
/// UDV: form_id,udv_id,varname,values,type
/// Columns in the UDV table have special meaning:
/// - type can be one of `FromValuesList`, `Constant`, or `RunDefined`
/// - if type is `FromValuesList`, column `value` will be a `\n`-separated list of values
/// - if type is `Constant`, column `value` will be its value
/// - if type is `RunDefined`, columns `value` will be `NULL`.
impl HgFormDef {
    /// Add new form definition to the database
    /// Each form will be used as a template for actual runs
    pub async fn new_form_def(
        new_formdef: HgFormDefSubmission,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        // Insert into Formdef table

        if new_formdef.version <= 0 {
            return Err(ModelError::FormError(String::from(
                "La version doit être définie et supérieure ou égale à 1",
            )));
        }
        if new_formdef.form_name.is_empty() {
            return Err(ModelError::FormError(String::from(
                "Le formulaire doit avoir un nom",
            )));
        }
        if new_formdef.pipeline_name.is_empty() {
            return Err(ModelError::FormError(String::from(
                "Pas de pipeline défini",
            )));
        }
        if new_formdef.launcher_name.is_empty() {
            return Err(ModelError::FormError(String::from(
                "Pas de launcher défini",
            )));
        }

        if sqlx::query(
            r#"
            SELECT COUNT(*) AS n FROM Formdef WHERE form_name = ? AND version = ?"#,
        )
        .bind(&new_formdef.form_name)
        .bind(new_formdef.version)
        .fetch_one(pool)
        .await?
        .try_get::<i64, &str>("n")?
            > 0
        {
            return Err(ModelError::FormError(String::from(
                "Un formulaire avec le même nom et la même version existe déjà. Abandon.",
            )));
        }

        let current_launcher_revision = {
            let config = config::get_mercure_config();
            let launcher_path = Path::new(&config.pipeline_dir)
                .join(&new_formdef.pipeline_name)
                .join("launchers")
                .join(&new_formdef.launcher_name);
            get_current_revision(&launcher_path)?
        };

        let form_id: i64 = sqlx::query(
            r#"
            INSERT INTO Formdef (pipeline_name, launcher_name, form_name, enabled, version, indir_type,latest_launcher_revision)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING form_id
            "#,
        )
        .bind(&new_formdef.pipeline_name)
        .bind(&new_formdef.launcher_name)
        .bind(&new_formdef.form_name)
        .bind(new_formdef.enabled)
        .bind(new_formdef.version)
        .bind(match new_formdef.indir_type {
            IndirType::BclDir => "BCL_DIR",
            IndirType::AnalysisDir => "ANALYSIS_DIR",
            IndirType::OntDir => "ONT_DIR",
        })
        .bind(current_launcher_revision)
        .fetch_one(pool)
        .await?
        .try_get(0usize)?;
        // Insert into UDV table
        // Insert user_defined_vars into UDV table
        for (varname, udv) in new_formdef.user_defined_vars.unwrap_or_default() {
            match udv {
                UserDefinedVar::FromValuesList { allowed } => {
                    let values = allowed.join("\n");
                    sqlx::query(
                        r#"
                        INSERT INTO UDV (form_id, varname, default_values, type)
                        VALUES (?, ?, ?, 'FromValuesList')
                        "#,
                    )
                    .bind(form_id)
                    .bind(varname)
                    .bind(values)
                    .execute(pool)
                    .await?;
                }
                UserDefinedVar::Constant(val) => {
                    sqlx::query(
                        r#"
                        INSERT INTO UDV (form_id, varname, default_values, type)
                        VALUES (?, ?, ?, 'Constant')
                        "#,
                    )
                    .bind(form_id)
                    .bind(varname)
                    .bind(val)
                    .execute(pool)
                    .await?;
                }
                UserDefinedVar::RunDefined => {
                    sqlx::query(
                        r#"
                        INSERT INTO UDV (form_id, varname, default_values, type)
                        VALUES (?, ?, NULL, 'RunDefined')
                        "#,
                    )
                    .bind(form_id)
                    .bind(varname)
                    .execute(pool)
                    .await?;
                }
                _ => {
                    eprintln!("Should be unreachable");
                }
            }
        }

        // Prepare groups associations by clearing them
        sqlx::query(
            r#"
            DELETE FROM FormdefHasGroup WHERE form_id = ?
            "#,
        )
        .bind(form_id)
        .execute(pool)
        .await?;
        // Insert groups into FormdefHasGroup table
        for group in &new_formdef.groups {
            sqlx::query(
                r#"
                INSERT INTO FormdefHasGroup (form_id, group_id)
                VALUES (?, ?)
                "#,
            )
            .bind(form_id)
            .bind(group.id)
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    pub async fn get_formdef_from_id(
        pool: &SqlitePool,
        id: i64,
    ) -> Result<HgFormDef, super::ModelError> {
        if id < 0 {
            return Err(ModelError::FormError(String::from(
                "ID de formulaire invalide",
            )));
        }
        let row = sqlx::query(
            r#"
        SELECT * FROM Formdef WHERE form_id = ?
        "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;
        Self::formdef_from_row(&row, pool).await
    }

    pub async fn disable_form(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        UPDATE Formdef SET enabled=0 WHERE form_id = ?
        "#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn enable_form(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        UPDATE Formdef SET enabled=1 WHERE form_id = ?
        "#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_all_form_defs(
        pool: &SqlitePool,
    ) -> Result<Vec<HgFormListItem>, super::ModelError> {
        let all_rows: Vec<_> =
            sqlx::query(r#"SELECT f.form_id,f.form_name,f.enabled,f.version FROM Formdef f;"#)
                .fetch_all(pool)
                .await?
                .into_iter()
                .filter_map(|row| {
                    Some(HgFormListItem {
                        form_name: row.try_get("form_name").ok()?,
                        formid: row.try_get("form_id").ok()?,
                        enabled: row.try_get("enabled").ok()?,
                        version: row.try_get("version").ok()?,
                    })
                })
                .collect();
        Ok(all_rows)
    }

    /// Get all forms associated with a group, and those not associated with any group
    /// Returns a tuple of two vectors:
    /// - first vector: forms associated with the group, sorted by version
    /// - second vector: forms not associated with any group, sorted by version
    pub async fn get_form_list_items_for_group(
        pool: &SqlitePool,
        group_id: i64,
    ) -> Result<(Vec<HgFormListItem>, Vec<HgFormListItem>), super::ModelError> {
        let mut rows_with_group:Vec<_> = sqlx::query(
                r#"
        SELECT f.form_id,f.form_name,f.enabled,f.version,fg.group_id,g.group_name FROM Formdef f JOIN FormdefHasGroup fg ON f.form_id = fg.form_id JOIN Groups g ON g.group_id = fg.group_id WHERE fg.group_id = ?
        "#,
            ).bind(group_id)
            .fetch_all(pool)
            .await?.into_iter().filter_map(|row|Some(HgFormListItem{
                form_name: row.try_get("form_name").ok()?,
                formid: row.try_get("form_id").ok()?,
                enabled: row.try_get("enabled").ok()?,
                version: row.try_get("version").ok()?
            })).collect();
        let mut rows_without_group:Vec<_> = sqlx::query(
                r#"SELECT f.form_id,f.form_name,f.enabled,f.version FROM Formdef f LEFT JOIN FormdefHasGroup fg ON f.form_id = fg.form_id WHERE fg.form_id IS NULL;"#,
            )
            .fetch_all(pool)
            .await?.into_iter().filter_map(|row|Some(HgFormListItem{
                form_name: row.try_get("form_name").ok()?,
                formid: row.try_get("form_id").ok()?,
                enabled: row.try_get("enabled").ok()?,
                version: row.try_get("version").ok()?
            })).collect();
        rows_with_group.sort_by(|a, b| a.version.cmp(&b.version));
        rows_without_group.sort_by(|a, b| a.version.cmp(&b.version));
        Ok((rows_with_group, rows_without_group))
    }

    pub async fn set_form_groups(
        pool: &SqlitePool,
        form_id: i64,
        group_ids: Vec<i64>,
    ) -> Result<(), sqlx::Error> {
        // Supprime les associations existantes
        sqlx::query("DELETE FROM FormdefHasGroup WHERE form_id = ?")
            .bind(form_id)
            .execute(pool)
            .await?;
        // Ajoute les nouvelles associations
        for gid in group_ids {
            sqlx::query("INSERT INTO FormdefHasGroup (form_id, group_id) VALUES (?, ?)")
                .bind(form_id)
                .bind(gid)
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    async fn formdef_from_row(
        row: &sqlx::sqlite::SqliteRow,
        pool: &SqlitePool,
    ) -> Result<HgFormDef, super::ModelError> {
        let mut def = HgFormDef::default();
        let form_id: i64 = row.try_get("form_id")?;

        def.form_id = form_id;
        def.pipeline_name = row.try_get("pipeline_name")?;
        def.launcher_name = row.try_get("launcher_name")?;
        def.form_name = row.try_get("form_name")?;
        def.enabled = row.try_get("enabled")?;
        def.version = row.try_get("version")?;
        def.indir_type = match row.try_get::<String, _>("indir_type")?.as_str() {
            "ANALYSIS_DIR" => IndirType::AnalysisDir,
            "ONT_DIR" => IndirType::OntDir,
            "BCL_DIR" => IndirType::BclDir,
            _ => IndirType::BclDir,
        };
        def.latest_launcher_revision = row.try_get("latest_launcher_revision")?;

        def.groups = sqlx::query(
            r#"SELECT g.group_id,g.group_name FROM Groups g JOIN FormdefHasGroup fg ON g.group_id=fg.group_id WHERE fg.form_id = ? "#,
        )
        .bind(form_id)
        .fetch_all(pool).
        await?.
        iter().
        filter_map(
        |row|
        Some(Group{ id: row.try_get("group_id").ok()?, name:row.try_get("group_name").ok()? }))
        .collect();

        def.user_defined_vars = Some(
            sqlx::query(r#"SELECT * FROM UDV WHERE form_id = ?"#)
                .bind(form_id)
                .fetch_all(pool)
                .await?
                .iter()
                .filter_map(|row| {
                    let udv = match row.try_get("type").ok()? {
                        "FromValuesList" => (
                            row.try_get("varname").ok()?,
                            UserDefinedVar::FromValuesList {
                                allowed: row
                                    .try_get::<String, &str>("default_values")
                                    .ok()?
                                    .split("\n")
                                    .map(String::from)
                                    .collect(),
                            },
                        ),
                        "RunDefined" => (row.try_get("varname").ok()?, UserDefinedVar::RunDefined),
                        "Constant" => (
                            row.try_get("varname").ok()?,
                            UserDefinedVar::Constant(row.try_get("default_values").ok()?),
                        ),
                        _ => {
                            return None;
                        }
                    };
                    Some(udv)
                })
                .collect(),
        );

        Ok(def)
    }

    /// Trouve des formulaires compatibles avec un formulaire archivé
    ///
    /// Critères de compatibilité :
    /// - Même pipeline_name et launcher_name
    /// - Mêmes UDVs (variables définies par l'utilisateur)
    /// - La révision git actuelle du launcher correspond à celle enregistrée dans le formulaire
    /// - Le formulaire est activé
    pub async fn find_compatible_forms(
        archived_form: &HgFormDef,
        pool: &SqlitePool,
    ) -> Result<Vec<HgFormDef>, ModelError> {
        // Avant tout, trouver la révision git actuelle du launcher
        // Permet de ne lister que les formulaires à jour
        let current_revision =
            crate::launchers_check::get_current_launcher_revision_for_form(archived_form)?;

        // Ensuite, récupérer tous les formulaires avec le même pipeline et launcher
        let candidate_rows = sqlx::query(
            r#"
            SELECT * FROM Formdef
            WHERE pipeline_name = ? AND launcher_name = ? AND enabled = 1 AND form_id != ? AND latest_launcher_revision = ?
            ORDER BY version DESC
            "#,
        )
        .bind(&archived_form.pipeline_name)
        .bind(&archived_form.launcher_name)
        .bind(archived_form.form_id)
        .bind(current_revision.trim())
        .fetch_all(pool)
        .await?;

        let mut compatible_forms = Vec::new();

        for row in candidate_rows {
            match Self::formdef_from_row(&row, pool).await {
                Ok(form) => {
                    // Vérifier la compatibilité des UDVs
                    if Self::are_udvs_compatible(
                        &archived_form.user_defined_vars,
                        &form.user_defined_vars,
                    ) {
                        compatible_forms.push(form);
                    }
                }
                Err(_) => continue, // Ignorer les formulaires avec des erreurs
            }
        }

        Ok(compatible_forms)
    }

    /// Vérifie si deux ensembles de variables définies par l'utilisateur sont compatibles
    fn are_udvs_compatible(
        udvs1: &Option<HashMap<String, UserDefinedVar>>,
        udvs2: &Option<HashMap<String, UserDefinedVar>>,
    ) -> bool {
        match (udvs1, udvs2) {
            (None, None) => true,
            (Some(map1), Some(map2)) => {
                // Vérifier que les deux maps ont les mêmes clés et les mêmes types de variables
                if map1.len() != map2.len() {
                    return false;
                }

                for (key, var1) in map1 {
                    match map2.get(key) {
                        Some(var2) => {
                            if !Self::are_udv_types_compatible(var1, var2) {
                                return false;
                            }
                        }
                        None => return false, // Clé manquante
                    }
                }
                true
            }
            _ => false, // Un a des UDVs, l'autre non
        }
    }

    /// Vérifie si deux variables définies par l'utilisateur sont compatibles
    fn are_udv_types_compatible(var1: &UserDefinedVar, var2: &UserDefinedVar) -> bool {
        match (var1, var2) {
            (UserDefinedVar::RunDefined, UserDefinedVar::RunDefined) => true,
            (UserDefinedVar::Constant(_), UserDefinedVar::Constant(_)) => true,
            (
                UserDefinedVar::FromValuesList { allowed: allowed1 },
                UserDefinedVar::FromValuesList { allowed: allowed2 },
            ) => {
                // Les listes de valeurs doivent être identiques
                allowed1.len() == allowed2.len() && allowed1.iter().all(|v| allowed2.contains(v))
            }
            _ => false, // Types différents
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_print_hgformdef_as_json() {
        let mut user_defined_vars = HashMap::new();
        user_defined_vars.insert(
            "species".to_string(),
            UserDefinedVar::FromValuesList {
                allowed: vec!["human".to_string(), "mouse".to_string()],
            },
        );
        user_defined_vars.insert(
            "project".to_string(),
            UserDefinedVar::Constant("CancerStudy".to_string()),
        );
        user_defined_vars.insert("batch".to_string(), UserDefinedVar::RunDefined);

        let form_def = HgFormDef {
            pipeline_name: "RNASeq".to_string(),
            launcher_name: "Nextflow".to_string(),
            form_name: "RNASeqForm".to_string(),
            enabled: true,
            version: 1,
            groups: vec![],
            user_defined_vars: Some(user_defined_vars),
            indir_type: IndirType::BclDir,
            form_id: 0,
            latest_launcher_revision: None,
        };

        let json = serde_json::to_string_pretty(&form_def).unwrap();
        println!("{json}");
        // Optionally, assert that JSON contains expected fields
        assert!(json.contains("RNASeqForm"));
        assert!(json.contains("species"));
        assert!(json.contains("FromValuesList"));

        let roundtrip_test: HgFormDef = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip_test, form_def);
    }
}
