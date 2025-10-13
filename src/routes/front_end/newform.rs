use rocket::State;
use rocket::get;
use sqlx::SqlitePool;

use rocket_dyn_templates::{Template, context};

use crate::auth::Authenticated;

use crate::config::MercureConfig;
use crate::config::get_mercure_config;
use crate::models::Group;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn list_folders_with_launchers<P: AsRef<Path>>(base_dir: P) -> HashMap<String, Vec<String>> {
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

/// Route: /mercure/admin/newform
#[get("/newform")]
pub async fn newform_get(auth: Authenticated, pool: &State<SqlitePool>) -> Template {
    if !auth.user.is_admin {
        Template::render("errors/admin_only", context! {user_name:auth.user.username})
    } else {
        let groups = (Group::get_groups_with_ids(pool).await).unwrap_or_default();
        let config: MercureConfig = get_mercure_config();
        let pipelines_struct = list_folders_with_launchers(config.pipeline_dir);

        Template::render(
            "admin/newform",
            context! {
                pipelines_struct,
                groups,
                formid: None::<i64>,
                submit_string: "Créer",
                title: "Nouveau formulaire de pipeline"
            },
        )
    }
}
