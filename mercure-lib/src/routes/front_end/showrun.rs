use std::collections::HashSet;
use std::ops::Not;

use rocket::State;
use rocket::get;
use sqlx::SqlitePool;

use crate::templates::{Template, context};

use crate::auth::Authenticated;
use crate::config::get_mercure_config;
use crate::models::Attempt;
use crate::models::Run;
use crate::models::RunStatus;
use crate::models::{Form, Group};

#[get("/show/run/<run_id>?<attempt_number>")]
pub async fn show_run_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    run_id: i64,
    attempt_number: Option<u32>,
) -> Template {
    // Get sequencers list for edit form
    let config = get_mercure_config();
    let run: Run = match Run::get_run_from_id(run_id, pool).await {
        Ok(run) => run,
        Err(e) => {
            return Template::render(
                "common/error",
                context! {
                    title=> "Erreur de la base de données",
                    h2=> format!("Impossible d'obtenir le run {}", run_id),
                    message=> e.to_string()
                },
            );
        }
    };
    let form: Form = match run.get_form().await {
        Ok(form) => form,
        Err(e) => {
            return Template::render(
                "common/error",
                context! { title=> "Erreur", h2=>format!("Impossible d'obtenir le formulaire pour le run {}",run_id),message=>e.to_string() },
            );
        }
    };
    let can_see_run = {
        if auth.user.is_admin {
            true
        } else {
            let form_groups: HashSet<i64> = form
                .get_group_ids(pool)
                .await
                .ok()
                .map(|v| v.into_iter().collect())
                .unwrap_or_default();

            let auth_groups: HashSet<i64> = match Group::get_user_groups(pool, auth.user.id).await {
                Ok(groups) => groups.iter().map(|grp| grp.id).collect(),
                Err(e) => {
                    return Template::render(
                        "common/error",
                        context! {title=> "Erreur de la base de données",
                        h2=> format!("Impossible d'obtenir les groupes de {} (vous)", auth.user.username),
                        message=> e.to_string()},
                    );
                }
            };
            let form_user_groups: HashSet<i64> = match Group::get_user_groups(pool, run.user_id)
                .await
            {
                Ok(groups) => groups.iter().map(|grp| grp.id).collect(),
                Err(e) => {
                    return Template::render(
                        "common/error",
                        context! {title=> "Erreur de la base de données",
                        h2=> format!("Impossible d'obtenir les groupes de {} (l'utilisateur qui a déclaré le run)", run.user.username),
                        message=> e.to_string()},
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
        // Récupérer l'historique des tentatives pour la navigation
        let history = Attempt::list_attempts_for_run(run_id, pool)
            .await
            .unwrap_or_default();

        // MODE ÉDITION : Si le run est Idle ET qu'on demande la dernière tentative
        if run.status == RunStatus::Idle && attempt_number.is_none() {
            let attempt = Attempt::get_hypothetic_attempt(&run);

            Template::render(
                "common/idlerun",
                context! {
                    run=> &run,
                    form=> &run.form,
                    attempt=> &attempt,
                    history=> history,
                    sequenceurs_list=> sequenceurs_list,
                    samplesheet_adn_static_name=> samplesheet_adn_static_name,
                    samplesheet_arn_static_name=> samplesheet_arn_static_name,
                    metadata_static_name=> metadata_static_name,
                    user_defined_vars_json=> &user_defined_vars_json,
                    user=> auth.user,
                },
            )
        } else {
            // MODE VISUALISATION : Run non Idle OU tentative spécifique demandée
            let attempt_number = attempt_number.unwrap_or(run.attempt_count);

            // On cherche la tentative demandée
            let attempt = match Attempt::get_attempt_from_number(attempt_number, run_id, pool).await
            {
                Ok(a) => a,
                Err(e) => {
                    return Template::render(
                        "common/error",
                        context! {
                            title=> "Tentative introuvable",
                            h2=> format!("Impossible d'obtenir la tentative {} pour le run {}", attempt_number, run_id),
                            message=> e.to_string()
                        },
                    );
                }
            };

            // On affiche le template correspondant au statut de la TENTATIVE (et non du Run)
            match attempt.status {
                RunStatus::Pending => Template::render(
                    "common/pendingrun",
                    context! {
                        run=> &run,
                        form=> &run.form,
                        attempt=> &attempt,
                        history=> &history,
                        samplesheet_adn_static_name=> samplesheet_adn_static_name,
                        samplesheet_arn_static_name=> samplesheet_arn_static_name,
                        metadata_static_name=> metadata_static_name,
                        user_defined_vars_json=> &user_defined_vars_json,
                        user=> auth.user,
                        is_pipeline_archived=> pipeline_is_archived,
                    },
                ),
                RunStatus::Running => Template::render(
                    "common/runningrun",
                    context! {
                        run=> &run,
                        form=> &run.form,
                        attempt=> &attempt,
                        history=> &history,
                        samplesheet_adn_static_name=> samplesheet_adn_static_name,
                        samplesheet_arn_static_name=> samplesheet_arn_static_name,
                        metadata_static_name=> metadata_static_name,
                        user_defined_vars_json=> &user_defined_vars_json,
                        user=> auth.user,
                        is_pipeline_archived=> pipeline_is_archived,
                    },
                ),
                RunStatus::Success => Template::render(
                    "common/successrun",
                    context! {
                        run=> &run,
                        form=> &run.form,
                        attempt=> &attempt,
                        history=> &history,
                        samplesheet_adn_static_name=> samplesheet_adn_static_name,
                        samplesheet_arn_static_name=> samplesheet_arn_static_name,
                        metadata_static_name=> metadata_static_name,
                        user_defined_vars_json=> &user_defined_vars_json,
                        user=> auth.user,
                        is_pipeline_archived=> pipeline_is_archived,
                    },
                ),
                RunStatus::Failure(ref fail_reason) => Template::render(
                    "common/failurerun",
                    context! {
                        run=> &run,
                        attempt=> &attempt,
                        form=> &run.form,
                        history=> &history,
                        samplesheet_adn_static_name=> samplesheet_adn_static_name,
                        samplesheet_arn_static_name=> samplesheet_arn_static_name,
                        metadata_static_name=> metadata_static_name,
                        fail_reason=> fail_reason,
                        user_defined_vars_json=> &user_defined_vars_json,
                        user=> auth.user,
                        is_pipeline_archived=> pipeline_is_archived,
                    },
                ),
                RunStatus::Idle => Template::render(
                    "common/error",
                    context! {
                        title=> "Erreur logique",
                        h2=> "Erreur logique",
                        message=> "Une erreur logique est survenue, veuillez contacter votre administrateur système.\nUne tentative ne peut pas être en état 'A valider'"
                    },
                ),
            }
        }
    } else {
        Template::render(
            "common/error",
            context! {
                title=> "Accès refusé",
                h2=> "Accès refusé",
                message=> "Vous n'avez pas les permissions nécessaires pour voir ce run"
            },
        )
    }
}

#[get("/show/runs")]
pub async fn list_runs() -> Template {
    Template::render("common/listruns", context! {})
}

#[get("/search/run")]
pub async fn search_run() -> Template {
    Template::render("common/searchrun", context! {})
}
