use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;

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

/// Struct used to define a form template
/// Only read from JSON, defined within the browser
/// See newform.html.jinja2 for more info
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct HgFormDef {
    pub pipeline_name: String,
    pub launcher_name: String,
    pub form_name: String,
    pub enabled: bool,
    pub version: i32,
    pub groups: Vec<Group>,
    pub user_defined_vars: Option<HashMap<String, UserDefinedVar>>,
    pub indir_type: IndirType,
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
    pub async fn new_form_def(new_formdef: HgFormDef, pool: &SqlitePool) -> Result<(), ModelError> {
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
        .bind(&new_formdef.version)
        .fetch_one(pool)
        .await?
        .try_get::<i64, &str>("n")?
            > 0
        {
            return Err(ModelError::FormError(String::from(
                "Un formulaire avec le même nom et la même version existe déjà. Abandon.",
            )));
        }

        let form_id: i64 = sqlx::query(
            r#"
            INSERT INTO Formdef (pipeline_name, launcher_name, form_name, enabled, version, indir_type)
            VALUES (?, ?, ?, ?, ?, ?)
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
