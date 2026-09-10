use crate::templates::{Template, context};
use rocket::{State, get};
use serde_json::{self};
use sqlx::SqlitePool;

use crate::auth::Authenticated;
use crate::config::get_mercure_config;
use crate::launchers_check::{exists_launcher, is_pipeline_archived};
use crate::models::{HgFormDef, HgRun};
use std::fs::read_dir;

#[get("/runs/submit/<form_id>")]
pub async fn new_run_get(auth: Authenticated, form_id: i64, pool: &State<SqlitePool>) -> Template {
    let config = get_mercure_config();
    let sequenceurs_folder = config.sequencers_dir;

    // Fetch form definition to get user_defined_vars
    let form_def = match HgFormDef::get_formdef_from_id(pool, form_id).await {
        Ok(form_def) => form_def,
        Err(e) => {
            return Template::render(
                "common/error",
                context! {
                    title=>"Formulaire invalide",
                    h2=>"Formulaire invalide",
                    message=>format!("Erreur lors du chargement de la définition du formulaire: {}", e)
                },
            );
        }
    };

    // Si le fichier de launcher n'existe plus, afficher une erreur
    if !exists_launcher(&form_def.pipeline_name, &form_def.launcher_name).await {
        return Template::render(
            "common/error",
            context! {
                title=>"Launcher manquant",
                h2=>"Launcher manquant",
                message=>format!("Le launcher spécifié dans le formulaire n'existe plus: {}/launchers/{}.", form_def.pipeline_name, form_def.launcher_name)
            },
        );
    }

    // Vérifier si le pipeline est archivé (révision git différente)
    let pipeline_is_archived = is_pipeline_archived(&form_def);
    if pipeline_is_archived {
        return Template::render(
            "common/error",
            context! {
                title=> "Pipeline archivé",
                h2=> "Pipeline archivé",
                message=> format!("Impossible de créer un nouveau run avec ce formulaire: le pipeline a été archivé. Veuillez demander à votre administrateur de mettre à jour le formulaire. Numéro du formulaire: {}.", form_def.form_id)
            },
        );
    }

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

    let user_defined_vars_json = match serde_json::to_string(
        &form_def.user_defined_vars.clone().unwrap_or_default(),
    ) {
        Ok(json_str) => json_str,
        Err(e) => {
            return Template::render(
                "common/error",
                context! {
                    title=>"Formulaire invalide",
                    h2=>"Formulaire invalide",
                    message=>format!("Erreur lors du chargement des variables définies par l'utilisateur: {}", e)
                },
            );
        }
    };

    // If we get here, render the editrun template normally (optionally a warning could be added later)
    Template::render(
        "common/editrun",
        context! {
            run=> None::<HgRun>,
            user=> auth.user,
            form_id,
            run_id=> None::<i64>,
            sequenceurs_list,
            user_defined_vars_json,
            indir_type=> &form_def.indir_type,
            form=> &form_def,
        },
    )
}
