use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::serde::json::Json;
use std::path::PathBuf;

use crate::config::get_mercure_config;
use crate::routes::ApiResponse;

#[derive(FromForm)]
pub struct UploadForm<'r> {
    pub file: TempFile<'r>,
    pub file_name_base: &'r str,
}

#[rocket::post("/upload", data = "<form>")]
pub async fn upload_post(mut form: Form<UploadForm<'_>>) -> Json<ApiResponse<String>> {
    let config = get_mercure_config();

    // Get upload directory
    let upload_dir = PathBuf::from(config.upload_dir);

    // Ensure directory exists
    if !upload_dir.exists() && std::fs::create_dir_all(&upload_dir).is_err() {
        return Json(ApiResponse::error("Failed to create upload directory"));
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
    let path_str = filepath.to_string_lossy().to_string();
    Json(ApiResponse::success(path_str))
}
