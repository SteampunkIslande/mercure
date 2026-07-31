use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::form::{HgFormDefSubmission, IndirType, UserDefinedVar};

/// The type of a user-defined variable in a form definition.
///
/// - `File`: An existing file the user should upload.
/// - `Choice`: A value selected among several predefined options.
/// - `Constant`: A fixed value (not editable by the user). Allows the same template
///   to be reused in multiple forms with different constant values.
/// - `Value`: Free-text input from the user.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum VariableType {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "choice")]
    Choice { choices: Vec<String> },
    #[serde(rename = "constant")]
    Constant { value: String },
    #[serde(rename = "value")]
    Value,
}

/// A single variable definition in a YAML form definition.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct VariableDef {
    pub name: String,
    #[serde(flatten)]
    pub var_type: VariableType,
}

/// A single form definition parsed from YAML.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FormYaml {
    pub name: String,
    pub template: String,
    #[serde(default)]
    pub dev_mode: bool,
    pub dev_branch: Option<String>,
    #[serde(default)]
    pub variables: Vec<VariableDef>,
}

/// The top-level YAML file containing a list of form definitions.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FormYamlFile {
    pub forms: Vec<FormYaml>,
}

impl FormYamlFile {
    /// Parse a YAML string into a FormYamlFile.
    pub fn from_str(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Load and parse a YAML file from the filesystem.
    pub fn from_file(path: &Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Self::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// A pipeline directory with its associated forms parsed from YAML.
#[derive(Debug, Serialize, Clone)]
pub struct PipelineWithForms {
    pub pipeline_name: String,
    pub forms: Vec<FormYaml>,
}

/// List all pipeline directories in the pipelines repo that contain a forms.yaml file.
pub fn list_pipelines_with_forms(pipelines_dir: &Path) -> Vec<PipelineWithForms> {
    let mut result = Vec::new();

    if let Ok(entries) = std::fs::read_dir(pipelines_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let pipeline_name = entry.file_name().to_string_lossy().to_string();
                let forms_yaml_path = entry.path().join("forms.yaml");

                if forms_yaml_path.exists() {
                    match FormYamlFile::from_file(&forms_yaml_path) {
                        Ok(yaml_file) => {
                            result.push(PipelineWithForms {
                                pipeline_name,
                                forms: yaml_file.forms,
                            });
                        }
                        Err(e) => {
                            eprintln!(
                                "Erreur lors de la lecture de {}: {}",
                                forms_yaml_path.display(),
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    result.sort_by(|a, b| a.pipeline_name.cmp(&b.pipeline_name));
    result
}

/// Convert a `FormYaml` into a `HgFormDefSubmission` for insertion into the DB.
pub fn form_yaml_to_submission(
    form_yaml: &FormYaml,
    pipeline_name: &str,
    version: i32,
    commit_hash: Option<String>,
) -> HgFormDefSubmission {
    let mut user_defined_vars = HashMap::new();

    for var in &form_yaml.variables {
        let udv = match &var.var_type {
            VariableType::File => UserDefinedVar::File,
            VariableType::Choice { choices } => UserDefinedVar::Choice {
                choices: choices.clone(),
            },
            VariableType::Constant { value } => UserDefinedVar::Constant(value.clone()),
            VariableType::Value => UserDefinedVar::Value,
        };
        user_defined_vars.insert(var.name.clone(), udv);
    }

    HgFormDefSubmission {
        pipeline_name: pipeline_name.to_string(),
        launcher_name: String::new(),
        form_name: form_yaml.name.clone(),
        enabled: true,
        groups: vec![],
        user_defined_vars: Some(user_defined_vars),
        indir_type: IndirType::BclDir,
        template_path: Some(form_yaml.template.clone()),
        dev_mode: form_yaml.dev_mode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml() {
        let yaml = r#"
forms:
  - name: "RNASeq Standard"
    template: "templates/rnaseq.sh.j2"
    dev_mode: false
    variables:
      - name: "species"
        type: "choice"
        choices: ["human", "mouse"]
      - name: "project"
        type: "constant"
        value: "CancerStudy"
      - name: "samplesheet"
        type: "file"
      - name: "batch"
        type: "value"

  - name: "RNASeq Dev"
    template: "templates/rnaseq.sh.j2"
    dev_mode: true
    dev_branch: "feature/new-aligner"
    variables:
      - name: "species"
        type: "choice"
        choices: ["human", "mouse", "rat"]
      - name: "project"
        type: "constant"
        value: "PilotStudy"
      - name: "samplesheet"
        type: "file"
"#;
        let parsed = FormYamlFile::from_str(yaml).unwrap();
        assert_eq!(parsed.forms.len(), 2);

        let form0 = &parsed.forms[0];
        assert_eq!(form0.name, "RNASeq Standard");
        assert_eq!(form0.template, "templates/rnaseq.sh.j2");
        assert!(!form0.dev_mode);
        assert!(form0.dev_branch.is_none());
        assert_eq!(form0.variables.len(), 4);

        assert_eq!(form0.variables[0].name, "species");
        assert_eq!(
            form0.variables[0].var_type,
            VariableType::Choice {
                choices: vec!["human".to_string(), "mouse".to_string()]
            }
        );

        assert_eq!(form0.variables[1].name, "project");
        assert_eq!(
            form0.variables[1].var_type,
            VariableType::Constant {
                value: "CancerStudy".to_string()
            }
        );

        assert_eq!(form0.variables[2].name, "samplesheet");
        assert_eq!(form0.variables[2].var_type, VariableType::File);

        assert_eq!(form0.variables[3].name, "batch");
        assert_eq!(form0.variables[3].var_type, VariableType::Value);

        let form1 = &parsed.forms[1];
        assert_eq!(form1.name, "RNASeq Dev");
        assert!(form1.dev_mode);
        assert_eq!(form1.dev_branch.as_deref(), Some("feature/new-aligner"));
        assert_eq!(form1.variables.len(), 3);
    }
}
