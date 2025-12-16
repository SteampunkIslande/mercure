use rocket::{State, get};
use rocket_dyn_templates::{Template, context};
use serde_json::{self};
use sqlx::SqlitePool;

use crate::auth::Authenticated;
use crate::config::get_mercure_config;
use crate::models::{HgFormDef, HgRun};
use std::fs::read_dir;

#[get("/runs/submit/<form_id>")]
pub async fn new_run_get(auth: Authenticated, form_id: i64, pool: &State<SqlitePool>) -> Template {
    let config = get_mercure_config();
    let sequenceurs_folder = config.sequencers_dir;

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
    let form_def = match HgFormDef::get_formdef_from_id(pool, form_id).await {
        Ok(form_def) => form_def,
        Err(e) => {
            return Template::render(
                "common/error",
                context! {
                    title:"Formulaire invalide",
                    h2:"Formulaire invalide",
                    message:format!("Erreur lors du chargement de la définition du formulaire: {}", e)
                },
            );
        }
    };
    let user_defined_vars = form_def.user_defined_vars.unwrap_or_default();
    let user_defined_vars_json = match serde_json::to_string(&user_defined_vars) {
        Ok(udv_string) => udv_string,
        Err(e) => {
            return Template::render(
                "common/error",
                context! {
                    title:"Formulaire invalide",
                    h2:"Formulaire invalide",
                    message:format!("Erreur lors du chargement des variables définies par l'utilisateur: {}", e)
                },
            );
        }
    };

    Template::render(
        "common/newrun",
        context! {
            run: None::<HgRun>,
            user: auth.user,
            form_id,
            run_id: None::<i64>,
            sequenceurs_list,
            user_defined_vars_json,
            indir_type: form_def.indir_type
        },
    )
}
