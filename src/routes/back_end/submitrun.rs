use rocket::serde::json::Json;
use rocket::{State, post};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::Authenticated;

use crate::models::HgRunSubmission;
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
