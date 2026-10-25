use crate::models::RunProgress;
use crate::pipeline_exec::RunProgressCache;
use crate::routes::ApiResponse;
use rocket::serde::json::Json;
use rocket::{State, get};

#[get("/watch/<run_id>/<attempt_number>")]
pub async fn watch(
    cache: &State<RunProgressCache>,
    run_id: i64,
    attempt_number: u32,
) -> Json<ApiResponse<RunProgress>> {
    Json(ApiResponse::success(
        cache.get_or_default(run_id, attempt_number).await,
    ))
}
