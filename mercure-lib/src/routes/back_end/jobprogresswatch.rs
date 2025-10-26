use crate::pipeline_exec::watch_log;
use rocket::get;

use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::json;

use std::path::PathBuf;

#[get("/watch/<job_id>/<attempt_number>")]
pub async fn watch(job_id: i64, attempt_number: i64) -> EventStream![] {
    let config = crate::config::get_mercure_config();

    let logs_folder = PathBuf::from(&config.logs_dir);

    let stream = watch_log(job_id, attempt_number, logs_folder).await;

    EventStream! {
        for await info in stream {
            match info {
                Ok(job_info) => {
                    yield Event::json(&json!(job_info)).event("update");
                }
                Err(e) => {
                    yield Event::data(format!("Erreur: {e}")).event("error");
                }
            }
        }
    }
}
