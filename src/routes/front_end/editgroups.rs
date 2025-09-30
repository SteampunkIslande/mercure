use crate::{auth::Authenticated, models::User};
use rocket::{State, get};
use rocket_dyn_templates::{Template, context};
use sqlx::SqlitePool;

/// Route to show the admin a page to register new user.
#[get("/groupedit/<user_id>")]
pub async fn edit_groups_for_user(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    user_id: i64,
) -> Template {
    if !auth.user.is_admin {
        return Template::render("errors/admin_only", context! {});
    }
    let user = User::find_by_id(user_id, pool).await.unwrap_or_default();
    Template::render("admin/groups_list", context! {user_id,user})
}
