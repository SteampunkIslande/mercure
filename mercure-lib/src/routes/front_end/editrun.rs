use std::collections::HashSet;
use std::fs::read_dir;
use std::ops::Not;

use rocket::State;
use rocket::get;
use sqlx::SqlitePool;

use rocket_dyn_templates::{Template, context};

use crate::auth::Authenticated;
use crate::config::get_mercure_config;
use crate::models::Group;
use crate::models::HgRun;
use crate::models::RunStatus;

#[get("/editrun/<run_id>")]
pub async fn edit_run_get(auth: Authenticated, pool: &State<SqlitePool>, run_id: i64) -> Template {
    let run: HgRun = match HgRun::get_run_from_id(run_id, pool).await {
        Ok(run) => run,
        Err(e) => {
            return Template::render(
                "common/error",
                context! {
                    title: "Erreur de la base de données",
                    h2: format!("Impossible d'obtenir le run {}", run_id),
                    message: e.to_string()
                },
            );
        }
    };
    let can_see_run = {
        if auth.user.is_admin {
            true
        } else {
            let form_groups: HashSet<i64> = run.form.groups.iter().map(|grp| grp.id).collect();
            let auth_groups: HashSet<i64> = match Group::get_user_groups(pool, auth.user.id).await {
                Ok(groups) => groups.iter().map(|grp| grp.id).collect(),
                Err(e) => {
                    return Template::render(
                        "common/error",
                        context! {title: "Erreur de la base de données",
                        h2: format!("Impossible d'obtenir les groupes de {} (vous)", auth.user.username),
                        message: e.to_string()},
                    );
                }
            };
            let form_user_groups: HashSet<i64> = match Group::get_user_groups(pool, run.user.id)
                .await
            {
                Ok(groups) => groups.iter().map(|grp| grp.id).collect(),
                Err(e) => {
                    return Template::render(
                        "common/error",
                        context! {title: "Erreur de la base de données",
                        h2: format!("Impossible d'obtenir les groupes de {} (l'utilisateur qui a déclaré le run)", run.user.username),
                        message: e.to_string()},
                    );
                }
            };
            // If there is a common group between the groups the form was declared for, the user that declared the run, and the authenticated user's groups, then you can see/edit the run
            intersection::hash_set::intersection([form_groups, auth_groups, form_user_groups])
                .is_empty()
                .not()
        }
    };

    if can_see_run {
        match run.status {
            RunStatus::Idle => {
                // Get sequencers list for edit form
                let config = get_mercure_config();
                let sequenceurs_folder = config.sequencers_dir;

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

                // Serialize user_defined_vars for JavaScript
                let user_defined_vars_json = match serde_json::to_string(
                    &run.form.user_defined_vars.clone().unwrap_or_default(),
                ) {
                    Ok(json_str) => json_str,
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
                    "common/editrun",
                    context! {
                        run: &run,
                        user: auth.user,
                        form_id: &run.form.form_id,
                        run_id: &run.run_id,
                        sequenceurs_list,
                        user_defined_vars_json,
                        indir_type: &run.form.indir_type,
                        form: &run.form,
                    },
                )
            }
            status => {
                let status_str = match status {
                    RunStatus::Running => "Run en cours",
                    RunStatus::Success => "Run terminé",
                    RunStatus::Failure(_) => "Run échoué",
                    _ => "Run non éditable",
                };
                Template::render(
                    "common/error",
                    context! {
                        title: status_str,
                        h2: format!("Impossible d'éditer un {}.", status_str.to_lowercase()),
                        message: "Seuls les runs à l'état 'A valider' peuvent être édités."
                    },
                )
            }
        }
    } else {
        Template::render(
            "common/error",
            context! {
                title: "Accès refusé",
                h2: "Accès refusé",
                message: "Vous n'avez pas les permissions nécessaires pour éditer ce run"
            },
        )
    }
}
