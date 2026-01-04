use rocket::serde::json::Json;
use rocket::{State, get, post};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::Authenticated;
use crate::launchers_check::is_pipeline_archived;

use crate::models::HgRun;
use crate::models::HgRunEdit;
use crate::models::HgRunSubmission;
use crate::models::analysis;
use crate::models::hgrun;

/// Route: /mercure/api/newrun
#[post("/newrun", data = "<form>")]
pub async fn newrun_post(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    form: Json<HgRunSubmission>,
) -> Json<ApiResponse<Value>> {
    match hgrun::HgRun::new_run(form.0, pool).await {
        Ok(run_id) => Json(ApiResponse::success(
            json!({"message":"Run créé avec succès!","run_id":run_id}),
        )),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}

/// Route: /mercure/api/validate/<run_id>
#[post("/validate/<run_id>")]
pub async fn validate_run_post(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    run_id: i64,
) -> Json<ApiResponse<Value>> {
    // Vérifier d'abord si le pipeline est archivé
    match HgRun::get_run_from_id(run_id, pool).await {
        Ok(run) => {
            if is_pipeline_archived(&run.form) {
                return Json(ApiResponse::error(
                    "Impossible de valider ce run: le pipeline utilise une version archivée. La révision git du launcher a changé depuis la création du formulaire.".to_string()
                ));
            }
        }
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "Erreur lors de la récupération du run: {e}"
            )));
        }
    }

    match analysis::validate_form(run_id, pool).await {
        Ok(_) => Json(ApiResponse::success(
            json!({"message":"Run validé avec succès!","run_id":run_id}),
        )),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}

#[get("/retry/<run_id>")]
pub async fn retry_run_get(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    run_id: i64,
) -> Json<ApiResponse<Value>> {
    // Vérifier d'abord si le pipeline est archivé
    match HgRun::get_run_from_id(run_id, pool).await {
        Ok(run) => {
            if is_pipeline_archived(&run.form) {
                return Json(ApiResponse::error(
                    "Impossible de relancer ce run: le pipeline utilise une version archivée. La révision git du launcher a changé depuis la création du formulaire.".to_string()
                ));
            }
        }
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "Erreur lors de la récupération du run: {e}"
            )));
        }
    }

    match analysis::relaunch_run(run_id, pool).await {
        Ok(_) => Json(ApiResponse::success(json!({"data":true}))),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}

/// Route: /mercure/api/editrun/
#[post("/editrun", data = "<form>")]
pub async fn editrun_post(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    form: Json<HgRunEdit>,
) -> Json<ApiResponse<Value>> {
    let run_id = form.run_id;

    // Vérifier d'abord si le pipeline est archivé
    match HgRun::get_run_from_id(run_id, pool).await {
        Ok(run) => {
            if is_pipeline_archived(&run.form) {
                return Json(ApiResponse::error(
                    "Impossible de modifier ce run: le pipeline utilise une version archivée. La révision git du launcher a changé depuis la création du formulaire.".to_string()
                ));
            }
        }
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "Erreur lors de la récupération du run: {e}"
            )));
        }
    }

    match hgrun::HgRun::edit_run(form.0, pool).await {
        Ok(_) => Json(ApiResponse::success(
            json!({"message":"Run modifié avec succès!","run_id":run_id}),
        )),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}
