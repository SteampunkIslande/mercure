use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::Authenticated;

use crate::models::{HgRun, HgRunSubmission};

/// Route: /mercure/api/newrun
#[post("/newrun", data = "<form>")]
pub async fn newrun_post(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    form: Json<HgRunSubmission>,
) -> Json<ApiResponse<String>> {
    match HgRun::new_run(form.0, pool).await {
        Ok(()) => Json(ApiResponse::success("Run créé avec succès!".to_string())),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}
