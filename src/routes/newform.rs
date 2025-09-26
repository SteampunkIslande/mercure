use rocket::State;
use rocket::serde::json::Json;
use rocket::{get, post};
use sqlx::SqlitePool;

use rocket_dyn_templates::{Template, context};

use super::ApiResponse;
use crate::auth::Authenticated;

use crate::models::{Group, HgFormDef};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn list_folders_with_launchers<P: AsRef<Path>>(base_dir: P) -> HashMap<String, Vec<String>> {
    let mut result = HashMap::new();

    if let Ok(entries) = fs::read_dir(&base_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let folder_name = entry.file_name().to_string_lossy().to_string();
                let launchers_path = path.join("launchers");
                let mut launchers = Vec::new();

                if launchers_path.is_dir() {
                    if let Ok(launcher_entries) = fs::read_dir(&launchers_path) {
                        for launcher_entry in launcher_entries.flatten() {
                            if let Some(name) = launcher_entry.file_name().to_str() {
                                launchers.push(name.to_string());
                            }
                        }
                    }
                }

                result.insert(folder_name, launchers);
            }
        }
    }

    result
}

#[get("/admin/newform")]
pub async fn newform_get(auth: Authenticated, pool: &State<SqlitePool>) -> Template {
    if !auth.user.is_admin {
        Template::render("errors/admin_only", context! {user_name:auth.user.username})
    } else {
        let groups = match Group::get_groups_with_ids(pool).await {
            Ok(groups) => groups,
            Err(_) => Vec::new(), // Return empty vector on error
        };
        if groups.is_empty() {
            return Template::render("admin/newgroup_redirect", context! {});
        }
        let pipelines_struct = list_folders_with_launchers("pipelines");

        Template::render(
            "admin/newform",
            context! {
                pipelines_struct,
                groups
            },
        )
    }
}

/// Route: /mercure/api/newform
#[post("/newform", data = "<form>")]
pub async fn newform_post(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    form: Json<HgFormDef>,
) -> Json<ApiResponse<String>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error(
            "You are not allowed to create new form, only admins can!".to_string(),
        ));
    }
    match HgFormDef::new_form_def(form.0, pool).await {
        Ok(()) => Json(ApiResponse::success("".to_string())),
        Err(e) => Json(ApiResponse::error(format!("{e}"))),
    }
}
