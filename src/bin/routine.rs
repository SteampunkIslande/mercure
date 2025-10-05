use sqlx;
use thiserror::Error;
use tokio::runtime::Runtime;

use mercure::models::HgAttempt;

#[derive(Error, Debug)]
enum RoutineError {
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
}

fn main() -> Result<(), RoutineError> {
    // Create the runtime
    let rt = Runtime::new().unwrap();

    rt.block_on(async || -> Result<(), RoutineError> {
        println!("hello");
        Ok(())
    }())
}
