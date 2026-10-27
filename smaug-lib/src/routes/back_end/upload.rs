use rocket::State;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::serde::json::Json;
use std::path::PathBuf;

use crate::auth::AuthError;
use crate::config::SmaugConfig;
use crate::routes::ApiResponse;

#[derive(FromForm)]
pub struct UploadForm<'r> {
    pub file: TempFile<'r>,
    pub file_name_base: &'r str,
}

#[rocket::post("/upload", data = "<form>")]
pub async fn upload_post(
    mut form: Form<UploadForm<'_>>,
    config: &State<SmaugConfig>,
) -> Result<Json<ApiResponse<String>>, AuthError> {
    // Get upload directory
    let upload_dir = PathBuf::from(&config.upload_dir);

    // If the upload dir doesn't exist, this is an error (admin should have created it!)
    if !upload_dir.exists() {
        return Ok(Json(ApiResponse::error(
            "Le dossier d'upload n'existe pas !",
        )));
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
        return Ok(Json(ApiResponse::error(
            "Impossible d'enregistrer le fichier",
        )));
    }

    // Return the path
    let path_str = filepath.display().to_string();
    Ok(Json(ApiResponse::success(path_str)))
}
