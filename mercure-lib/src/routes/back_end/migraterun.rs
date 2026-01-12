use rocket::serde::json::Json;
use rocket::{State, get};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use super::super::ApiResponse;
use crate::auth::Authenticated;
use crate::models::{HgFormDef, HgRun, HgRunSubmission};

/// API pour créer un nouveau run à partir d'un run archivé en utilisant un formulaire compatible
#[get("/api/migrate/run?<from_run_id>&<to_form_id>")]
pub async fn migrate_run_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    from_run_id: i64,
    to_form_id: i64,
) -> Json<ApiResponse<Value>> {
    // Récupérer le run original
    let original_run = match HgRun::get_run_from_id(from_run_id, pool).await {
        Ok(run) => run,
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "Impossible de récupérer le run original: {}",
                e
            )));
        }
    };

    // Vérifier que l'utilisateur peut voir ce run (même logique que dans showrun.rs)
    let can_see_run = auth.user.is_admin || {
        use crate::models::Group;
        use std::collections::HashSet;
        use std::ops::Not;

        let form_groups: HashSet<i64> = original_run.form.groups.iter().map(|grp| grp.id).collect();
        let auth_groups: HashSet<i64> = match Group::get_user_groups(pool, auth.user.id).await {
            Ok(groups) => groups.iter().map(|grp| grp.id).collect(),
            Err(_) => HashSet::new(),
        };
        let form_user_groups: HashSet<i64> =
            match Group::get_user_groups(pool, original_run.user.id).await {
                Ok(groups) => groups.iter().map(|grp| grp.id).collect(),
                Err(_) => HashSet::new(),
            };
        intersection::hash_set::intersection([form_groups, auth_groups, form_user_groups])
            .is_empty()
            .not()
    };

    if !can_see_run {
        return Json(ApiResponse::error(
            "Vous n'avez pas les permissions pour accéder à ce run".to_string(),
        ));
    }

    // Récupérer le formulaire cible
    let target_form = match HgFormDef::get_formdef_from_id(pool, to_form_id).await {
        Ok(form) => form,
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "Impossible de récupérer le formulaire cible: {}",
                e
            )));
        }
    };

    // Vérifier que le formulaire cible est compatible
    let compatible_forms = match HgFormDef::find_compatible_forms(&original_run.form, pool).await {
        Ok(forms) => forms,
        Err(e) => {
            return Json(ApiResponse::error(format!(
                "Erreur lors de la recherche de formulaires compatibles: {}",
                e
            )));
        }
    };

    if !compatible_forms.iter().any(|f| f.form_id == to_form_id) {
        return Json(ApiResponse::error(
            "Le formulaire cible n'est pas compatible avec le run original".to_string(),
        ));
    }

    // Créer le nouveau run avec les données de l'original mais le nouveau formulaire
    let new_run_submission = HgRunSubmission {
        form_id: target_form.form_id,
        user_id: auth.user.id, // L'utilisateur qui effectue la migration devient le propriétaire
        run_name: format!("{} (migré)", original_run.run_name),
        run_date: original_run.run_date.clone(),
        run_sequencer: original_run.run_sequencer.clone(),
        run_flowcellid: original_run.run_flowcellid.clone(),
        sample_sheet_adn_path: original_run.sample_sheet_adn_path.clone(),
        sample_sheet_arn_path: original_run.sample_sheet_arn_path.clone(),
        metadata_path: original_run.metadata_path.clone(),
        user_defined_vars: original_run.user_defined_vars.clone(),
        indir: original_run.indir.clone(),
        outdir: original_run.outdir.clone(),
    };

    match HgRun::new_run(new_run_submission, pool).await {
        Ok(new_run_id) => Json(ApiResponse::success(json!({
            "message": format!("Nouveau run créé avec succès à partir du run archivé. ID: {}", new_run_id),
            "new_run_id": new_run_id
        }))),
        Err(e) => Json(ApiResponse::error(format!(
            "Erreur lors de la création du nouveau run: {}",
            e
        ))),
    }
}
