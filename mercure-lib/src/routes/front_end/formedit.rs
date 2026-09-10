use crate::config::{MercureConfig, get_mercure_config};
use crate::models::HgFormDef;
use rocket::State;
use rocket::get;
use sqlx::SqlitePool;

use crate::templates::{Template, context};

use crate::auth::Authenticated;

use crate::models::Group;

use super::newform::list_folders_with_launchers;

#[get("/editform/<formid>")]
pub async fn editform_get(auth: Authenticated, pool: &State<SqlitePool>, formid: i64) -> Template {
    if !auth.user.is_admin {
        Template::render(
            "errors/admin_only",
            context! {user_name=>auth.user.username},
        )
    } else {
        let groups = (Group::get_groups_with_ids(pool).await).unwrap_or_default();
        let config: MercureConfig = get_mercure_config();
        let pipelines_struct = list_folders_with_launchers(config.pipeline_dir);

        Template::render(
            "admin/editform",
            context! {
                pipelines_struct,
                groups,
                formid,
                edit_mode=> true,
                title=> "Editer un formulaire de pipeline"
            },
        )
    }
}

#[get("/editform/groups?<form_id>")]
pub async fn edit_groups_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    form_id: i64,
) -> Template {
    if !auth.user.is_admin {
        Template::render(
            "errors/admin_only",
            context! {user_name=>auth.user.username},
        )
    } else {
        let groups = Group::get_groups_with_ids(pool).await.unwrap_or_default();
        let form_groups = Group::get_groups_for_form(pool, form_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|g| g.id)
            .collect::<Vec<i64>>();
        let form: HgFormDef = HgFormDef::get_formdef_from_id(pool, form_id)
            .await
            .unwrap_or_default();
        Template::render(
            "admin/formgroupedit",
            context! {groups, form_id, form_groups, form},
        )
    }
}

#[get("/show/forms")]
pub async fn show_forms_get(auth: Authenticated, pool: &State<SqlitePool>) -> Template {
    let user_groups: Option<Vec<Group>> = Group::get_user_groups(pool, auth.user.id).await.ok();

    if !auth.user.is_admin {
        Template::render(
            "errors/admin_only",
            context! {user_name=>auth.user.username},
        )
    } else {
        Template::render(
            "admin/showforms",
            context! {user_groups=>user_groups, is_admin=>auth.user.is_admin},
        )
    }
}
