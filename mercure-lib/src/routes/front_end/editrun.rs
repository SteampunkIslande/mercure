use std::collections::HashSet;
use std::ops::Not;

use anyhow::Context;
use rocket::State;
use rocket::get;
use sqlx::SqlitePool;

use crate::routes::frontend::FrontendError;
use crate::templates::{Template, context};

use crate::auth::Authenticated;
use crate::models::Group;
use crate::models::{Run, RunStatus};

#[get("/editrun/<run_id>")]
pub async fn edit_run_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    run_id: i64,
) -> Result<Template, FrontendError> {
    let run: Run = Run::get_run_from_id(run_id, pool).await?;
    let form = run.get_form().await?;
    let can_see_run = {
        if auth.user.is_admin {
            true
        } else {
            let form_groups: HashSet<i64> = form.get_group_ids(pool).await?.into_iter().collect();
            let auth_groups: HashSet<i64> = Group::get_user_groups(pool, auth.user.id)
                .await?
                .iter()
                .map(|grp| grp.id)
                .collect();
            let form_user_groups: HashSet<i64> = Group::get_user_groups(pool, run.user_id)
                .await
                .context(format!(
                    "Impossible d'obtenir les groupes de {} (l'utilisateur qui a déclaré le run)",
                    run.get_user(pool).await?.username
                ))?
                .iter()
                .map(|grp| grp.id)
                .collect();
            // If there is a common group between the groups the form was declared for, the user that declared the run, and the authenticated user's groups, then you can see/edit the run
            intersection::hash_set::intersection([form_groups, auth_groups, form_user_groups])
                .is_empty()
                .not()
        }
    };

    if can_see_run {
        match run.status {
            RunStatus::Idle => Ok(Template::render(
                "common/editrun",
                context! {
                    run=> &run,
                    user=> &auth.user,
                    form=> &form,
                    run_id=> &run_id,
                },
            )),
            status => {
                let status_str = match status {
                    RunStatus::Running => "Run en cours",
                    RunStatus::Success => "Run terminé",
                    RunStatus::Failure(_) => "Run échoué",
                    _ => "Run non éditable",
                };
                Ok(Template::render(
                    "common/error",
                    context! {
                        title=> status_str,
                        h2=> format!("Impossible d'éditer un {}.", status_str.to_lowercase()),
                        message=> "Seuls les runs à l'état 'A valider' peuvent être édités."
                    },
                ))
            }
        }
    } else {
        Ok(Template::render(
            "common/error",
            context! {
                title=> "Accès refusé",
                h2=> "Accès refusé",
                message=> "Vous n'avez pas les permissions nécessaires pour éditer ce run"
            },
        ))
    }
}
