use rocket::form::{Form, FromForm};
use rocket::serde::json::Json;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs::read_to_string;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config::get_smaug_config;
use crate::routes::ApiResponse;
use crate::utils::correct_samplesheet;

#[derive(FromForm)]
pub struct FileCheckForm<'r> {
    pub file_name: &'r str,
}

/// Errors of the samplesheet itself are reported to the user in the `check` field
/// ("perfect", "fixable", "invalid") over HTTP 200; only an unreadable file is a
/// `Failure` payload.
#[rocket::post("/samplesheet/listsamples", data = "<form>")]
pub async fn list_samples_from_samplesheet(
    form: Form<FileCheckForm<'_>>,
) -> Json<ApiResponse<Value>> {
    let samplesheet_content = match read_to_string(form.file_name) {
        Ok(content) => content,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };

    // An invalid samplesheet is a *value* reported to the user, not a failure.
    match correct_samplesheet(&samplesheet_content) {
        Ok((sample_names, _)) => Json(ApiResponse::success(json!({"sample_names":sample_names}))),
        Err(e) => Json(ApiResponse::success(
            json!({"sample_names":[],"error":e.to_string()}),
        )),
    }
}

#[rocket::post("/samplesheet/check", data = "<form>")]
pub async fn check_samplesheet(form: Form<FileCheckForm<'_>>) -> Json<ApiResponse<Value>> {
    let samplesheet_content = match read_to_string(form.file_name) {
        Ok(content) => content,
        Err(e) => return Json(ApiResponse::error(e.to_string())),
    };
    let config = get_smaug_config();
    // Get upload directory
    let upload_dir = PathBuf::from(config.upload_dir);

    let samplesheet_rel_path = match Path::new(form.file_name).strip_prefix(upload_dir) {
        Ok(p) => p,
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "La samplesheet n'est pas stockée dans le dossier upload {}",
                e
            )));
        }
    };

    match correct_samplesheet(&samplesheet_content) {
        Ok((sample_names, fix)) => {
            let unique_sample_count = sample_names
                .iter()
                .map(|s| s.as_str())
                .collect::<HashSet<&str>>()
                .len();

            if fix == samplesheet_content {
                Json(ApiResponse::success(json!(
                {
                    "message": r#"<div class="success-msg" style="animation:none;"><span class="material-icons icon-align" style="color:green;">check</span>La samplesheet est parfaitement valide.</div>"#,

                    "check": "perfect",

                    "sample_names": sample_names,

                    "unique_sample_count": unique_sample_count,

                    "file_name": form.file_name
                })))
            } else {
                // A write failure does not lose the analysis: the samplesheet stays
                // "fixable" and the user is told to upload it again.
                let message = match std::fs::File::create(form.file_name)
                    .and_then(|mut f| f.write(fix.as_bytes()))
                {
                    Ok(_) => format!(
                        r#"<div class="warning-msg" style="animation:none;"><span class="material-icons icon-align" style="color:orange;">warning</span>La samplesheet a été corrigée car elle contenait une ou des erreur(s), veuillez tracer l'incident. La samplesheet modifiée est disponible à <a href="/uploads/{}" download>cette adresse</a></div>"#,
                        samplesheet_rel_path.to_string_lossy()
                    ),
                    Err(e) => {
                        log::warn!(
                            "Impossible d'enregistrer la samplesheet corrigée {}: {e}",
                            form.file_name
                        );
                        r#"<div class="error-msg" style="animation:none;"><span class="material-icons icon-align" style="color:red;">error</span>La samplesheet aurait pu être corrigée, mais l'enregistrement de la copie corrigée a échoué. Veuillez charger le fichier à nouveau.</div>"#.to_string()
                    }
                };

                Json(ApiResponse::success(json!(
                {
                    "message": message,

                    "check": "fixable",

                    "sample_names": sample_names,

                    "unique_sample_count": unique_sample_count,

                    "file_name": form.file_name
                })))
            }
        }
        Err(e) => Json(ApiResponse::success(json!(
        {
            "message": format!(r#"<div class="error-msg" style="animation:none;"><span class="material-icons icon-align" style="color:red;">error</span>La samplesheet n'est pas une samplesheet Illumina valide. Erreur:<br>{}"#, e),

            "check": "invalid",

            "sample_names": [],

            "unique_sample_count": 0,

            "file_name": form.file_name
        }))),
    }
}
