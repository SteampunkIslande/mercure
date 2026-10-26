use rocket::serde::json::Json;
use rocket::{State, get};

use crate::pipeline_exec::{JobStatus, SqueueCache};
use crate::routes::ApiResponse;

#[get("/squeue?<job_name>")]
pub async fn get_squeue(
    job_name: &str,
    cache: &State<SqueueCache>,
) -> Json<ApiResponse<Vec<JobStatus>>> {
    match cache.get(job_name).await {
        // A cache/squeue miss is a normal answer (the job may not be queued any
        // more), not a server failure: keep it in the payload, over HTTP 200.
        Some(jobs) => Json(ApiResponse::success(jobs)),
        None => Json(ApiResponse::error(
            "Impossible d'interroger squeue pour ce nom de job",
        )),
    }
}
