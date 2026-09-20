use rocket::serde::json::Json;
use rocket::{State, get, post};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::Authenticated;

use crate::models::Run;
use crate::models::analysis;
use crate::models::hgrun;

/// Route: /mercure/api/newrun
#[post("/newrun", data = "<form>")]
pub async fn newrun_post(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    form: Json<Run>,
) -> Json<ApiResponse<Value>> {
    match hgrun::Run::new_run(form.0, pool).await {
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
    if let Err(e) = Run::get_run_from_id(run_id, pool).await {
        return Json(ApiResponse::error(format!(
            "Erreur lors de la récupération du run: {e}"
        )));
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
    if let Err(e) = Run::get_run_from_id(run_id, pool).await {
        return Json(ApiResponse::error(format!(
            "Erreur lors de la récupération du run: {e}"
        )));
    }

    match analysis::relaunch_run(run_id, pool).await {
        Ok(_) => Json(ApiResponse::success(json!({"data":true}))),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}

/// Route: /mercure/api/editrun/
#[post("/editrun", data = "<run>")]
pub async fn editrun_post(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    run: Json<Run>,
) -> Json<ApiResponse<Value>> {
    let run_id = match run.run_id {
        Some(run_id) => run_id,
        None => return Json(ApiResponse::error("Aucun run ID spécifié!")),
    };
    if let Err(e) = Run::get_run_from_id(run_id, pool).await {
        return Json(ApiResponse::error(format!(
            "Erreur lors de la récupération du run: {e}"
        )));
    }

    match hgrun::Run::edit_run(run_id, run.0.user_defined_vars, pool).await {
        Ok(_) => Json(ApiResponse::success(
            json!({"message":"Run modifié avec succès!","run_id":run_id}),
        )),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}
