use std::ops::Deref;

use crate::utils::parse_launcher;
use rocket::State;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde_json::Value;
use sqlx::Row;
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::Authenticated;

use crate::models::ModelError;
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

#[get("/parselauncher?<pipeline>&<launcher>")]
pub async fn parse_launcher_endpoint(
    pipeline: String,
    launcher: String,
) -> Json<ApiResponse<Value>> {
    // On vérifie que le pipeline et le launcher sont valides
    let config: crate::config::MercureConfig = crate::config::get_mercure_config();

    let launcher_abs_path = std::path::Path::new(&config.pipeline_dir)
        .join(&pipeline)
        .join("launchers")
        .join(&launcher);
    if !launcher_abs_path.exists() {
        Json(ApiResponse::error(format!(
            "Le launcher spécifié n'existe pas: {}",
            launcher_abs_path.display()
        )))
    } else {
        let launcher_text = match std::fs::read_to_string(&launcher_abs_path) {
            Ok(s) => s,
            Err(e) => {
                return Json(ApiResponse::error(format!(
                    "Erreur lors de la lecture du launcher {}: {}",
                    launcher_abs_path.display(),
                    e
                )));
            }
        };
        match parse_launcher(&launcher_text)
            .and_then(|vars| serde_json::to_value(&vars).map_err(|e| e.to_string()))
        {
            Ok(vars) => Json(ApiResponse::success(vars)),
            Err(e) => Json(ApiResponse::error(format!(
                "Erreur lors de l'analyse du launcher {}: {}",
                launcher_abs_path.display(),
                e
            ))),
        }
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
