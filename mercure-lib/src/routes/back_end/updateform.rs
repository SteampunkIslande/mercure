use crate::auth::Authenticated;
use crate::config;
use crate::launchers_check::get_current_revision;
use crate::models::{FormStatusInfo, HgFormDef};
use crate::routes::ApiResponse;
use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::SqlitePool;
use std::path::Path;

#[get("/update-form-status")]
pub async fn forms_update_status_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
) -> Json<ApiResponse<Vec<FormStatusInfo>>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "Accès refusé : seuls les administrateurs peuvent voir cette ressource.".to_string(),
        ));
    } else {
        match HgFormDef::list_forms_archive_status(pool).await {
            Ok(status_list) => Json(ApiResponse::success(status_list)),
            Err(e) => Json(ApiResponse::error(format!(
                "Erreur lors de la récupération des statuts des formulaires : {}",
                e
            ))),
        }
    }
}

#[get("/update-form/<form_id>")]
pub async fn forms_list_updates_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    form_id: i64,
) -> Json<ApiResponse<String>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "Accès refusé : seuls les administrateurs peuvent voir cette ressource.".to_string(),
        ));
    }
    let form: HgFormDef = match HgFormDef::get_formdef_from_id(pool, form_id).await {
        Ok(f) => f,
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "Erreur lors de la récupération du formulaire : {}",
                e
            )));
        }
    };

    let new_revision = {
        let config = config::get_mercure_config();
        let launcher_path = Path::new(&config.pipeline_dir)
            .join(&form.pipeline_name)
            .join("launchers")
            .join(&form.launcher_name);
        match get_current_revision(&launcher_path) {
            Ok(rev) => rev,
            Err(e) => {
                return Json(ApiResponse::error(format!(
                    "Erreur lors de la récupération de la révision du launcher : {}",
                    e
                )));
            }
        }
    };

    match HgFormDef::update_latest_launcher_revision(pool, form_id, new_revision).await {
        Ok(_) => Json(ApiResponse::success("Mise à jour effectuée.".to_string())),
        Err(e) => Json(ApiResponse::error(format!(
            "Erreur lors de la mise à jour : {}",
            e
        ))),
    }
}
