use crate::auth::Authenticated;
use crate::config;
use crate::launchers_check::get_current_revision;
use crate::models::{FormStatusInfo, HgFormDef};
use crate::routes::ApiResponse;
use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct PaginationInfo {
    pub current_page: u64,
    pub per_page: u64,
    pub total_items: u64,
    pub total_pages: u64,
    pub has_next_page: bool,
    pub has_previous_page: bool,
}

#[derive(Serialize, Deserialize)]
pub struct PaginatedFormsData {
    pub items: Vec<FormStatusInfo>,
    pub pagination: PaginationInfo,
}

#[get("/update-form-status?<page>&<per_page>")]
pub async fn forms_update_status_paginated_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    page: Option<u64>,
    per_page: Option<u64>,
) -> Json<ApiResponse<PaginatedFormsData>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "Accès refusé : seuls les administrateurs peuvent voir cette ressource.".to_string(),
        ));
    }

    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(10).clamp(1, 100); // Limite entre 1 et 100

    match HgFormDef::list_forms_archive_status_paginated(pool, page, per_page).await {
        Ok((status_list, total_count)) => {
            let total_pages = if per_page == 0 {
                0
            } else {
                (total_count + per_page - 1) / per_page
            };

            let pagination_info = PaginationInfo {
                current_page: page,
                per_page,
                total_items: total_count,
                total_pages,
                has_next_page: page < total_pages,
                has_previous_page: page > 1,
            };

            let paginated_data = PaginatedFormsData {
                items: status_list,
                pagination: pagination_info,
            };

            Json(ApiResponse::success(paginated_data))
        }
        Err(e) => Json(ApiResponse::error(format!(
            "Erreur lors de la récupération des statuts des formulaires : {}",
            e
        ))),
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
