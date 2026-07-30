use rocket::State;
use rocket::get;
use sqlx::SqlitePool;

use rocket_dyn_templates::{Template, context};

use crate::auth::Authenticated;
use crate::models::Group;

#[get("/importform")]
pub async fn importform_get(auth: Authenticated, pool: &State<SqlitePool>) -> Template {
    if !auth.user.is_admin {
        Template::render("errors/admin_only", context! {user_name:auth.user.username})
    } else {
        let groups = (Group::get_groups_with_ids(pool).await).unwrap_or_default();

        Template::render(
            "admin/importform",
            context! {
                groups,
                title: "Importer un formulaire depuis YAML"
            },
        )
    }
}
