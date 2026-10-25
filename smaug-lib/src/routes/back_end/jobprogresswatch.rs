use crate::models::RunProgress;
use crate::routes::ApiResponse;
use rocket::serde::json::Json;
use rocket::{State, get};
use sqlx::SqlitePool;

#[get("/watch/<run_id>/<attempt_number>")]
pub async fn watch(
    pool: &State<SqlitePool>,
    run_id: i64,
    attempt_number: u32,
) -> Json<ApiResponse<RunProgress>> {
    match RunProgress::get_or_default(run_id, attempt_number, pool).await {
        Ok(progress) => Json(ApiResponse::success(progress)),
        Err(e) => Json(ApiResponse::error(format!(
            "Impossible de lire la progression du run: {}",
            e
        ))),
    }
}
