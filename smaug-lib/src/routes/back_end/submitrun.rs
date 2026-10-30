use anyhow::Context;
use rocket::serde::json::Json;
use rocket::{State, get, post};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::{AuthError, Authenticated};

use crate::models::Run;
use crate::models::RunDefinitionError;
use crate::models::analysis;
use crate::models::hgrun;

/// Route: /smaug/api/newrun
#[post("/newrun", data = "<run>")]
pub async fn newrun_post(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    mut run: Json<Run>,
) -> Result<Json<ApiResponse<Value>>, AuthError> {
    // No matter what was posted, we overwrite the run submitter here anyways
    run.0.user_id = auth.user.id;

    let run_id = hgrun::Run::new_run(run.0, pool).await?;
    Ok(Json(ApiResponse::success(
        json!({"message":"Run créé avec succès!","run_id":run_id}),
    )))
}

/// Route: /smaug/api/validate/<run_id>
#[post("/validate/<run_id>")]
pub async fn validate_run_post(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    run_id: i64,
) -> Result<Json<ApiResponse<Value>>, AuthError> {
    Run::get_run_from_id(run_id, pool).await?;
    analysis::validate_form(run_id, pool).await?;
    Ok(Json(ApiResponse::success(
        json!({"message":"Run validé avec succès!","run_id":run_id}),
    )))
}

#[get("/retry/<run_id>")]
pub async fn retry_run_get(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    run_id: i64,
) -> Result<Json<ApiResponse<Value>>, AuthError> {
    Run::get_run_from_id(run_id, pool).await?;
    if let Err(e) = analysis::relaunch_run(run_id, pool).await {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }
    Ok(Json(ApiResponse::success(
        json!({"message":"Le run est à nouveau à valider"}),
    )))
}

/// Route: /smaug/api/editrun/
#[post("/editrun", data = "<run>")]
pub async fn editrun_post(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    run: Json<Run>,
) -> Result<Json<ApiResponse<Value>>, AuthError> {
    let run_id = run.run_id.context(RunDefinitionError::MissingRunID)?;
    Run::get_run_from_id(run_id, pool).await?;
    hgrun::Run::edit_run(run_id, run.0.user_defined_vars, pool).await?;
    Ok(Json(ApiResponse::success(
        json!({"message":"Run modifié avec succès!","run_id":run_id}),
    )))
}
