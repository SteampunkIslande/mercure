use rocket::State;
use rocket::get;
use sqlx::SqlitePool;

use rocket_dyn_templates::{Template, context};

use crate::auth::Authenticated;

use crate::models::Group;

use super::newform::list_folders_with_launchers;

#[get("/editform/<formid>")]
pub async fn editform_get(auth: Authenticated, pool: &State<SqlitePool>, formid: i64) -> Template {
    if !auth.user.is_admin {
        Template::render("errors/admin_only", context! {user_name:auth.user.username})
    } else {
        let groups = (Group::get_groups_with_ids(pool).await).unwrap_or_default();
        let pipelines_struct = list_folders_with_launchers("pipelines");

        Template::render(
            "admin/newform",
            context! {
                pipelines_struct,
                groups,
                formid,
            },
        )
    }
}

#[get("/show/forms")]
pub async fn show_forms_get(auth: Authenticated, pool: &State<SqlitePool>) -> Template {
    let user_groups: Option<Vec<Group>> = Group::get_user_groups(pool, auth.user.id).await.ok();

    if !auth.user.is_admin {
        Template::render("errors/admin_only", context! {user_name:auth.user.username})
    } else {
        Template::render("admin/showforms", context! {user_groups:user_groups, is_admin:auth.user.is_admin})
    }
}
