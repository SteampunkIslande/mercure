use rocket::form::{Form, FromForm};
use rocket::serde::json::Json;
use serde_json::{Value, json};
use std::fs::read_to_string;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config::get_mercure_config;
use crate::routes::ApiResponse;
use crate::utils::correct_samplesheet;

#[derive(FromForm)]
pub struct FileCheckForm<'r> {
    pub file_name: &'r str,
}

#[rocket::post("/samplesheet/listsamples", data = "<form>")]
pub async fn list_samples_from_samplesheet(
    form: Form<FileCheckForm<'_>>,
) -> Json<ApiResponse<Value>> {
    let samplesheet_content = match read_to_string(form.file_name) {
        Ok(s) => s,
        Err(e) => {
            return Json(ApiResponse::error(e.to_string()));
        }
    };
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
        Ok(s) => s,
        Err(e) => {
            return Json(ApiResponse::error(e.to_string()));
        }
    };
    let config = get_mercure_config();
    // Get upload directory
    let upload_dir = PathBuf::from(config.upload_dir);

    let samplesheet_rel_path = match Path::new(form.file_name).strip_prefix(upload_dir) {
        Ok(p) => p,
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "La samplesheet n'est pas stockée dans le dossier upload {}",
                e.to_string()
            )));
        }
    };

    match correct_samplesheet(&samplesheet_content) {
        Ok((sample_names, fix)) => {
            if fix == samplesheet_content {
                Json(ApiResponse::success(json!(
                {
                    "message":r#"<div class="success-msg" style="animation:none;"><span class="material-icons icon-align" style="color:green;">check</span>La samplesheet est parfaitement valide.</div>"#,

                    "check":"perfect",

                    "sample_names":sample_names,

                    "file_name":form.file_name
                })))
            } else {
                match std::fs::File::create(form.file_name)
                    .and_then(|mut f| f.write(fix.as_bytes()))
                {
                    Ok(_) => Json(ApiResponse::success(json!(
                    {
                        "message":format!(r#"<div class="warning-msg" style="animation:none;"><span class="material-icons icon-align" style="color:orange;">warning</span>La samplesheet a pu être corrigée, aucune action requise de votre part. Le résultat est disponible à <a href="/uploads/{}" download>cette adresse</a></div>"#, samplesheet_rel_path.to_string_lossy()),

                        "check":"fixable",

                        "sample_names":sample_names,

                        "file_name":form.file_name
                    }))),
                    Err(_) => Json(ApiResponse::success(json!(
                    {
                        "message":r#"<div class="error-msg" style="animation:none;"><span class="material-icons icon-align" style="color:red;">error</span>La samplesheet aurait pu être corrigée, mais l'enregistrement de la copie corrigée a échoué. Veuillez charger le fichier à nouveau.</div>"#,

                        "check":"fixable",

                        "sample_names":sample_names,

                        "file_name":form.file_name
                    }))),
                }
            }
        }
        Err(e) => Json(ApiResponse::success(json!(
        {
            "message":format!(r#"<div class="error-msg" style="animation:none;"><span class="material-icons icon-align" style="color:red;">error</span>La samplesheet n'est pas une samplesheet Illumina valide. Erreur:<br>{}"#,e),

            "check":"invalid",

            "sample_names":[],

            "file_name":form.file_name
        }))),
    }
}
