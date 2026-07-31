use std::ops::Deref;

use rocket::State;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde_json::Value;
use sqlx::Row;
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::Authenticated;

use crate::models::ModelError;
use crate::models::form_yaml;
use crate::models::{HgFormDef, HgFormDefSubmission};

#[get("/nextversion?<formname>")]
pub async fn get_nextversion(
    formname: String,
    pool: &State<SqlitePool>,
) -> Json<ApiResponse<Value>> {
    Json(ApiResponse::success(
        match sqlx::query("SELECT COUNT(*) AS count FROM Formdef WHERE form_name = ?")
            .bind(&formname)
            .fetch_one(pool.deref())
            .await
            .and_then(|r| r.try_get::<i64, &str>("count"))
        {
            Ok(count) => Value::from(count + 1),
            Err(_) => Value::from(1),
        },
    ))
}

/// Route: /mercure/api/pipelines
/// Lists all pipeline directories with their available forms from forms.yaml
#[get("/pipelines")]
pub async fn list_pipelines(auth: Authenticated) -> Json<ApiResponse<Value>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "Accès refusé : seuls les administrateurs peuvent lister les pipelines.".to_string(),
        ));
    }

    let config = crate::config::get_mercure_config();
    let pipelines_dir = std::path::Path::new(&config.pipeline_dir);

    let pipelines = form_yaml::list_pipelines_with_forms(pipelines_dir);

    match serde_json::to_value(&pipelines) {
        Ok(v) => Json(ApiResponse::success(v)),
        Err(e) => Json(ApiResponse::error(format!(
            "Erreur lors de la sérialisation des pipelines: {e}"
        ))),
    }
}

/// Request body for importing a form from YAML
#[derive(rocket::serde::Deserialize)]
pub struct ImportFormRequest {
    pub pipeline_name: String,
    pub form_name: String,
    pub version: i32,
    pub groups: Vec<crate::models::Group>,
    pub commit_hash: Option<String>,
}

/// Route: /mercure/api/importform
/// Creates a form definition in the DB from a YAML form definition
#[post("/importform", data = "<req>")]
pub async fn import_form(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    req: Json<ImportFormRequest>,
) -> Json<ApiResponse<String>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "Accès refusé : seuls les administrateurs peuvent importer des formulaires."
                .to_string(),
        ));
    }

    let config = crate::config::get_mercure_config();
    let pipelines_dir = std::path::Path::new(&config.pipeline_dir);

    let forms_yaml_path = pipelines_dir.join(&req.pipeline_name).join("forms.yaml");
    let yaml_file = match form_yaml::FormYamlFile::from_file(&forms_yaml_path) {
        Ok(f) => f,
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "Erreur lors de la lecture du fichier forms.yaml: {e}"
            )));
        }
    };

    let form_yaml = match yaml_file.forms.iter().find(|f| f.name == req.form_name) {
        Some(f) => f,
        None => {
            return Json(ApiResponse::error(format!(
                "Le formulaire '{}' n'existe pas dans le pipeline '{}'",
                req.form_name, req.pipeline_name
            )));
        }
    };

    let mut submission = form_yaml::form_yaml_to_submission(
        form_yaml,
        &req.pipeline_name,
        req.version,
        req.commit_hash.clone(),
    );
    submission.groups = req.groups.clone();

    match HgFormDef::new_form_def(submission, pool).await {
        Ok(()) => Json(ApiResponse::success(
            "Formulaire importé avec succès!".to_string(),
        )),
        Err(e) => match e {
            ModelError::LauncherCheckError(e) => Json(ApiResponse::error(format!(
                "{e}\nVeuillez valider ou annuler ces modifications avant de créer un nouveau formulaire."
            ))),
            e => Json(ApiResponse::error(format!("{e}"))),
        },
    }
}

/// Route: /mercure/api/disable/<formid>
#[get("/disable/<formid>")]
pub async fn disable_form(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    formid: i64,
) -> Json<ApiResponse<String>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "You cannot disable a form, only admins can!".to_string(),
        ));
    }
    match HgFormDef::disable_form(pool, formid).await {
        Ok(()) => Json(ApiResponse::success(
            "Formulaire désactivé avec succès".to_string(),
        )),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}

/// Route: /mercure/api/enable/<formid>
#[get("/enable/<formid>")]
pub async fn enable_form(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    formid: i64,
) -> Json<ApiResponse<String>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "You cannot enable a form, only admins can!".to_string(),
        ));
    }
    match HgFormDef::enable_form(pool, formid).await {
        Ok(()) => Json(ApiResponse::success(
            "Formulaire activé avec succès".to_string(),
        )),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}

/// Route: /mercure/api/newform
#[post("/newform", data = "<form>")]
pub async fn newform_post(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    form: Json<HgFormDefSubmission>,
) -> Json<ApiResponse<String>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "You are not allowed to create new form, only admins can!".to_string(),
        ));
    }

    match HgFormDef::new_form_def(form.0, pool).await {
        Ok(()) => Json(ApiResponse::success(
            "Formulaire créé avec succès!".to_string(),
        )),
        Err(e) => match e {
            ModelError::LauncherCheckError(e) => Json(ApiResponse::error(format!(
                "{e}\nVeuillez valider ou annuler ces modifications avant de créer un nouveau formulaire."
            ))),
            e => Json(ApiResponse::error(format!("{e}"))),
        },
    }
}
