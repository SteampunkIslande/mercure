use std::fs::read_dir;

use rocket::{Config, State, get};
use rocket_dyn_templates::{Template, context};
use serde_json::{self, json};
use sqlx::SqlitePool;

use crate::auth::Authenticated;
use crate::config::MercureConfig;
use crate::models::HgFormDef;

#[get("/runs/submit/<form_id>")]
pub async fn new_run_get(auth: Authenticated, form_id: i64, pool: &State<SqlitePool>) -> Template {
    let config = Config::figment()
        .extract::<MercureConfig>()
        .unwrap_or_default();
    let sequenceurs_folder = config.sequencers_folder;

    // List directories at the top level of sequencers_folder
    let sequenceurs_list = read_dir(&sequenceurs_folder)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|entry| {
                    entry.ok().and_then(|e| {
                        if e.file_type().ok()?.is_dir() {
                            e.file_name().to_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    // Fetch form definition to get user_defined_vars
    let form_def = HgFormDef::get_formdef_from_id(pool, form_id)
        .await
        .unwrap_or_default();
    eprintln!("{:?}", form_def);
    let user_defined_vars = form_def.user_defined_vars.unwrap_or_default();
    let user_defined_vars_json = match serde_json::to_string(&user_defined_vars) {
        Ok(udv_string) => udv_string,
        Err(e) => format!("{}", json! ({"error":e.to_string()})),
    };

    Template::render(
        "common/newrun",
        context! { form_id, user: auth.user, sequenceurs_list, user_defined_vars_json },
    )
}
