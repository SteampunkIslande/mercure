use crate::pipeline_exec::watch_log;
use rocket::{Shutdown, get};

use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::json;

use std::path::PathBuf;

#[get("/watch/<job_id>/<attempt_number>")]
pub async fn watch(job_id: i64, attempt_number: i64, mut end: Shutdown) -> EventStream![] {
    let config = crate::config::get_mercure_config();

    let logs_folder = PathBuf::from(&config.logs_dir);

    let stream = watch_log(job_id, attempt_number, logs_folder).await;

    eprintln!(
        "Starting EventStream for job_id={} attempt_number={}",
        job_id, attempt_number
    );

    EventStream! {
        use rocket::futures::StreamExt;
        use tokio::pin;
        pin!(stream);
        loop {
            tokio::select! {
                info = stream.next() => {
                    match info {
                        Some(Ok(job_info)) => {
                            yield Event::json(&json!(job_info)).event("update");
                        }
                        Some(Err(e)) => {
                            eprintln!("Error while watching log for job_id={} attempt_number={}: {}", job_id, attempt_number, e);
                            break;
                        }
                        None => {
                            eprintln!("End of log stream for job_id={} attempt_number={}", job_id, attempt_number);
                            break;
                        }
                    }
                }
                _ = &mut end => {
                    eprintln!("Client déconnecté pour job_id={} attempt_number={}", job_id, attempt_number);
                    break;
                }
            }
        }
    }
}
