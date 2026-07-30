use minijinja::Environment;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

/// Renders a minijinja template with user-defined variables and Mercure pre-defined variables.
///
/// # Arguments
/// * `template_content` - The content of the .j2 template file
/// * `user_vars` - User-defined variable values (HashMap<String, String>)
/// * `mercure_indir` - Input directory for the run
/// * `mercure_outdir` - Output directory for the run
/// * `mercure_run_name` - User-defined name for the run
/// * `mercure_run_id` - Internal run ID
/// * `mercure_attempt_number` - Attempt number for this run
/// * `mercure_pipelines_dir` - Path to the cloned pipelines repo inside outdir
///
/// # Returns
/// The rendered script as a String, or an error message.
pub fn render_script_template(
    template_content: &str,
    user_vars: &HashMap<String, String>,
    mercure_indir: &str,
    mercure_outdir: &str,
    mercure_run_name: &str,
    mercure_run_id: i64,
    mercure_attempt_number: i64,
    mercure_pipelines_dir: &str,
) -> Result<String, minijinja::Error> {
    let env = Environment::new();
    let template = env.template_from_str(template_content)?;

    let mut context = json!({
        "mercure": {
            "indir": mercure_indir,
            "outdir": mercure_outdir,
            "run_name": mercure_run_name,
            "run_id": mercure_run_id,
            "attempt_number": mercure_attempt_number,
            "pipelines_dir": mercure_pipelines_dir,
        }
    });

    if let serde_json::Value::Object(ref mut map) = context {
        for (k, v) in user_vars {
            map.insert(k.clone(), serde_json::Value::String(v.clone()));
        }
    }

    template.render(context)
}

/// Loads a template file from the pipeline directory and renders it.
///
/// # Arguments
/// * `pipelines_dir` - The root pipelines directory
/// * `pipeline_name` - The pipeline subfolder name
/// * `template_path` - The relative path to the .j2 template file (relative to the pipeline dir)
/// * `user_vars` - User-defined variable values
/// * `mercure_*` - Mercure pre-defined variables
pub fn render_template_from_file(
    pipelines_dir: &Path,
    pipeline_name: &str,
    template_path: &str,
    user_vars: &HashMap<String, String>,
    mercure_indir: &str,
    mercure_outdir: &str,
    mercure_run_name: &str,
    mercure_run_id: i64,
    mercure_attempt_number: i64,
    mercure_pipelines_dir: &str,
) -> Result<String, TemplateRenderError> {
    let full_path = pipelines_dir.join(pipeline_name).join(template_path);
    let template_content = std::fs::read_to_string(&full_path)
        .map_err(|e| TemplateRenderError::IoError(e))?;

    render_script_template(
        &template_content,
        user_vars,
        mercure_indir,
        mercure_outdir,
        mercure_run_name,
        mercure_run_id,
        mercure_attempt_number,
        mercure_pipelines_dir,
    )
    .map_err(TemplateRenderError::RenderError)
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateRenderError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    RenderError(#[from] minijinja::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template() {
        let template = r#"#!/bin/bash
# Run: {{ mercure.run_name }}
# Input: {{ mercure.indir }}
# Output: {{ mercure.outdir }}

species="{{ species }}"
project="{{ project }}"
samplesheet="{{ samplesheet }}"

cd {{ mercure.pipelines_dir }}

bash pipeline.sh --species {{ species }} --project {{ project }} --samplesheet {{ samplesheet }} --indir {{ mercure.indir }} --outdir {{ mercure.outdir }}
"#;

        let mut user_vars = HashMap::new();
        user_vars.insert("species".to_string(), "human".to_string());
        user_vars.insert("project".to_string(), "CancerStudy".to_string());
        user_vars.insert("samplesheet".to_string(), "/uploads/samples.csv".to_string());

        let rendered = render_script_template(
            template,
            &user_vars,
            "/data/input",
            "/data/output",
            "TestRun",
            42,
            1,
            "/data/output/pipelines",
        )
        .unwrap();

        assert!(rendered.contains("Run: TestRun"));
        assert!(rendered.contains("Input: /data/input"));
        assert!(rendered.contains("Output: /data/output"));
        assert!(rendered.contains("species=\"human\""));
        assert!(rendered.contains("project=\"CancerStudy\""));
        assert!(rendered.contains("samplesheet=\"/uploads/samples.csv\""));
        assert!(rendered.contains("cd /data/output/pipelines"));
    }

    #[test]
    fn test_render_template_with_mercure_vars() {
        let template = r#"echo "Run ID: {{ mercure.run_id }}, Attempt: {{ mercure.attempt_number }}"
echo "Pipelines: {{ mercure.pipelines_dir }}""#;

        let user_vars = HashMap::new();
        let rendered = render_script_template(
            template,
            &user_vars,
            "/in",
            "/out",
            "MyRun",
            123,
            3,
            "/out/pipelines",
        )
        .unwrap();

        assert!(rendered.contains("Run ID: 123, Attempt: 3"));
        assert!(rendered.contains("Pipelines: /out/pipelines"));
    }
}
