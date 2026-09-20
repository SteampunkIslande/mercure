use rocket::State;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::serde::json::Json;
use std::path::PathBuf;

use crate::config::MercureConfig;
use crate::routes::ApiResponse;

#[derive(FromForm)]
pub struct UploadForm<'r> {
    pub file: TempFile<'r>,
    pub file_name_base: &'r str,
}

#[rocket::post("/upload", data = "<form>")]
pub async fn upload_post(
    mut form: Form<UploadForm<'_>>,
    config: &State<MercureConfig>,
) -> Json<ApiResponse<String>> {
    // Get upload directory
    let upload_dir = PathBuf::from(&config.upload_dir);

    // If the upload dir doesn't exist, this is an error (admin should have created it!)
    if !upload_dir.exists() {
        return Json(ApiResponse::error("Le dossier d'upload n'existe pas !"));
    }

    // Generate a unique filename. Alphabetical order is also creation time order.
    let filename = format!(
        "{}-{}",
        time::OffsetDateTime::now_utc().unix_timestamp(),
        form.file_name_base
    );
    let filepath = upload_dir.join(filename);

    // Persist the file
    if (form.file.move_copy_to(&filepath).await).is_err() {
        return Json(ApiResponse::error("Failed to save file"));
    }

    // Return the path
    let path_str = filepath.display().to_string();
    Json(ApiResponse::success(path_str))
}
