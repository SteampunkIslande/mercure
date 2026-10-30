use std::path::PathBuf;

use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::{State, get};

use crate::auth::Authenticated;
use crate::config::SmaugConfig;

/// Serves an uploaded file to authenticated users only.
#[get("/<path..>")]
pub async fn serve_upload(
    _auth: Authenticated,
    config: &State<SmaugConfig>,
    path: PathBuf,
) -> Result<NamedFile, Status> {
    if path.components().count() == 0 {
        return Err(Status::NotFound);
    }

    let upload_dir = PathBuf::from(&config.upload_dir)
        .canonicalize()
        .map_err(|_| Status::NotFound)?;
    let file_path = upload_dir
        .join(&path)
        .canonicalize()
        .map_err(|_| Status::NotFound)?;

    if !file_path.starts_with(&upload_dir) || !file_path.is_file() {
        return Err(Status::NotFound);
    }

    NamedFile::open(file_path)
        .await
        .map_err(|_| Status::NotFound)
}
