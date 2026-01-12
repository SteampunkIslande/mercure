use std::collections::HashSet;
use std::fs::read_dir;
use std::ops::Not;

use rocket::State;
use rocket::get;
use sqlx::SqlitePool;

use rocket_dyn_templates::{Template, context};

use crate::auth::Authenticated;
use crate::config::get_mercure_config;
use crate::launchers_check::{check_launcher_exists, is_pipeline_archived};
use crate::models::HgAttempt;
use crate::models::HgRun;
use crate::models::RunStatus;
use crate::models::{Group, HgFormDef};
use crate::utils::filename_to_static_served_name;

#[get("/show/run/<run_id>?<attempt_number>")]
pub async fn show_run_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    run_id: i64,
    attempt_number: Option<i64>,
) -> Template {
    // Get sequencers list for edit form
    let config = get_mercure_config();
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
        let form_def = &run.form;

        if !check_launcher_exists(&form_def.pipeline_name, &form_def.launcher_name).await {
            // Désactiver le formulaire si ce n'était pas déjà fait
            HgFormDef::disable_form(pool, form_def.form_id).await.ok();
            return Template::render(
                "common/error",
                context! {
                    title:"Launcher manquant",
                    h2:"Launcher manquant",
                    message:format!("Impossible d'éditer le run {}: le launcher spécifié dans le formulaire n'existe plus ({}/launchers/{}). Le formulaire correspondant a été désactivé.", run.run_id, form_def.pipeline_name, form_def.launcher_name)
                },
            );
        }

        // Vérifier si le pipeline est archivé
        let pipeline_is_archived = is_pipeline_archived(&form_def);

        // Si le pipeline est archivé, rechercher des formulaires compatibles
        let compatible_forms = if pipeline_is_archived {
            HgFormDef::find_compatible_forms(&form_def, pool)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Récupérer l'historique des tentatives pour la navigation
        let history = HgAttempt::list_attempts_for_run(run_id, pool)
            .await
            .unwrap_or_default();

        // Serialize user_defined_vars for JavaScript
        let user_defined_vars_json =
            match serde_json::to_string(&run.form.user_defined_vars.clone().unwrap_or_default()) {
                Ok(json_str) => json_str,
                Err(e) => format!("{{\"Error\": \"{}\"}}", e),
            };
        // MODE ÉDITION : Si le run est Idle ET qu'on demande la dernière tentative
        if run.status == RunStatus::Idle && attempt_number.is_none() {
            let sequenceurs_folder = config.sequencers_dir;

            let attempt = HgAttempt::get_hypothetic_attempt(&run);

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

            let samplesheet_adn_static_name = filename_to_static_served_name(
                &run.sample_sheet_adn_path,
                &config.upload_dir,
                "/uploads",
            );

            let samplesheet_arn_static_name = filename_to_static_served_name(
                &run.sample_sheet_arn_path,
                &config.upload_dir,
                "/uploads",
            );

            let metadata_static_name =
                filename_to_static_served_name(&run.metadata_path, &config.upload_dir, "/uploads");

            Template::render(
                "common/idlerun",
                context! {
                    run: &run,
                    form: &run.form,
                    attempt: &attempt,
                    history: history,
                    sequenceurs_list: sequenceurs_list,
                    samplesheet_adn_static_name: samplesheet_adn_static_name,
                    samplesheet_arn_static_name: samplesheet_arn_static_name,
                    metadata_static_name: metadata_static_name,
                    user_defined_vars_json: &user_defined_vars_json,
                    user: auth.user,
                    pipeline_is_archived: pipeline_is_archived,
                    compatible_forms: &compatible_forms,
                },
            )
        } else {
            // MODE VISUALISATION : Run non Idle OU tentative spécifique demandée
            let attempt_number = attempt_number.unwrap_or(run.attempt_count as i64);

            // On cherche la tentative demandée
            let attempt = match HgAttempt::get_attempt_from_number(attempt_number, run_id, pool)
                .await
            {
                Ok(a) => a,
                Err(e) => {
                    return Template::render(
                        "common/error",
                        context! {
                            title: "Tentative introuvable",
                            h2: format!("Impossible d'obtenir la tentative {} pour le run {}", attempt_number, run_id),
                            message: e.to_string()
                        },
                    );
                }
            };

            let samplesheet_adn_static_name = filename_to_static_served_name(
                &attempt.sample_sheet_adn_path,
                &config.upload_dir,
                "/uploads",
            );

            let samplesheet_arn_static_name = filename_to_static_served_name(
                &attempt.sample_sheet_arn_path,
                &config.upload_dir,
                "/uploads",
            );

            let metadata_static_name = filename_to_static_served_name(
                &attempt.metadata_path,
                &config.upload_dir,
                "/uploads",
            );

            // On affiche le template correspondant au statut de la TENTATIVE (et non du Run)
            match attempt.status {
                RunStatus::Pending => Template::render(
                    "common/pendingrun",
                    context! {
                        run: &run,
                        form: &run.form,
                        attempt: &attempt,
                        history: &history,
                        samplesheet_adn_static_name: samplesheet_adn_static_name,
                        samplesheet_arn_static_name: samplesheet_arn_static_name,
                        metadata_static_name: metadata_static_name,
                        user_defined_vars_json: &user_defined_vars_json,
                        user: auth.user,
                        pipeline_is_archived: pipeline_is_archived,
                        compatible_forms: &compatible_forms,
                    },
                ),
                RunStatus::Running => Template::render(
                    "common/runningrun",
                    context! {
                        run: &run,
                        form: &run.form,
                        attempt: &attempt,
                        history: &history,
                        samplesheet_adn_static_name: samplesheet_adn_static_name,
                        samplesheet_arn_static_name: samplesheet_arn_static_name,
                        metadata_static_name: metadata_static_name,
                        user_defined_vars_json: &user_defined_vars_json,
                        user: auth.user,
                        pipeline_is_archived: pipeline_is_archived,
                        compatible_forms: &compatible_forms,
                    },
                ),
                RunStatus::Success => Template::render(
                    "common/successrun",
                    context! {
                        run: &run,
                        form: &run.form,
                        attempt: &attempt,
                        history: &history,
                        samplesheet_adn_static_name: samplesheet_adn_static_name,
                        samplesheet_arn_static_name: samplesheet_arn_static_name,
                        metadata_static_name: metadata_static_name,
                        user_defined_vars_json: &user_defined_vars_json,
                        user: auth.user,
                        pipeline_is_archived: pipeline_is_archived,
                        compatible_forms: &compatible_forms,
                    },
                ),
                RunStatus::Failure(ref fail_reason) => Template::render(
                    "common/failurerun",
                    context! {
                        run: &run,
                        attempt: &attempt,
                        form: &run.form,
                        history: &history,
                        samplesheet_adn_static_name: samplesheet_adn_static_name,
                        samplesheet_arn_static_name: samplesheet_arn_static_name,
                        metadata_static_name: metadata_static_name,
                        fail_reason: fail_reason,
                        user_defined_vars_json: &user_defined_vars_json,
                        user: auth.user,
                        pipeline_is_archived: pipeline_is_archived,
                        compatible_forms: &compatible_forms,
                    },
                ),
                RunStatus::Idle => Template::render(
                    "common/error",
                    context! {
                        title: "Erreur logique",
                        h2: "Erreur logique",
                        message: "Une erreur logique est survenue, veuillez contacter votre administrateur système.\nUne tentative ne peut pas être en état 'A valider'"
                    },
                ),
            }
        }
    } else {
        Template::render(
            "common/error",
            context! {
                title: "Accès refusé",
                h2: "Accès refusé",
                message: "Vous n'avez pas les permissions nécessaires pour voir ce run"
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
