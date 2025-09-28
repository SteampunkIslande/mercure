use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;

use crate::models::ModelError;

use super::groups::Group;

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum UserDefinedVar {
    FromValuesList {
        allowed: Vec<String>,
    },
    Constant(String),
    #[default]
    RunDefined,
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
}

/// Created by users.
/// On any user's home page, there is a list of runs submitted by the user
/// There is also a button that the user can press to get to route '/newrun/groupname'
///
/// This form is what is submitted by the user when they are on the '/newrun/groupname' GET endpoint
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HgForm {
    // These fields are from the FormDef
    pub pipeline_name: String,
    pub launcher_name: String,
    pub form_name: String,
    pub user_defined_vars: HashMap<String, String>,

    // These fields are common to all forms
    pub run_date: String,
    pub run_sequencer: String,
    pub run_flowcellid: String,
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
        new_formdef: HgFormDef,
        pool: &SqlitePool,
    ) -> Result<(), super::ModelError> {
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
                "Pas de pipeline défini!",
            )));
        }
        if new_formdef.launcher_name.is_empty() {
            return Err(ModelError::FormError(String::from(
                "Pas de launcher défini!",
            )));
        }

        // Make sure the (form_name,version) is unique!
        if let Ok(_) = sqlx::query(
            r#"
            SELECT * FROM Formdef WHERE form_name = ? AND version = ?
            "#,
        )
        .bind(&new_formdef.form_name)
        .bind(&new_formdef.version)
        .fetch_one(pool)
        .await
        {
            return Err(ModelError::FormError(String::from(
                "Un formulaire avec le même nom et la même version existe déjà. Veuillez augmenter le numéro de version",
            )));
        }

        let form_id: i64 = sqlx::query(
            r#"
            INSERT INTO Formdef (pipeline_name, launcher_name, form_name, enabled, version)
            VALUES (?, ?, ?, ?, ?)
            RETURNING form_id
            "#,
        )
        .bind(&new_formdef.pipeline_name)
        .bind(&new_formdef.launcher_name)
        .bind(&new_formdef.form_name)
        .bind(new_formdef.enabled)
        .bind(new_formdef.version)
        .fetch_one(pool)
        .await?
        .try_get(0usize)?;

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

        if let Some(user_defined_vars) = &new_formdef.user_defined_vars {
            // Insert user_defined_vars into UDV table
            for (varname, udv) in user_defined_vars {
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
                }
            }
        }
        Ok(())
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
