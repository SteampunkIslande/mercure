use crate::templates::{Template, context};
use crate::{auth::Authenticated, models::User};
use rocket::{State, get};
use sqlx::SqlitePool;

#[get("/dashboard")]
pub async fn admin_dashboard_get(auth: Authenticated) -> Option<Template> {
    if !auth.user.is_admin {
        return Some(Template::render("errors/admin_only", context! {}));
    }
    Some(Template::render(
        "admin/dashboard",
        context! {user=>auth.user},
    ))
}

/// The admin route to change a user's password
#[get("/passedit/<user_id>")]
pub async fn password_edit_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    user_id: i64,
) -> Option<Template> {
    if user_id != auth.user.id && !auth.user.is_admin {
        return Some(Template::render("errors/admin_only", context! {}));
    }
    let edited_user = User::find_by_id(user_id, pool).await.ok()?;
    Some(Template::render(
        "admin/passedit",
        context! {auth_user=> auth.user, edited_user=>edited_user},
    ))
}

/// Route to show the admin a page to register new user.
#[get("/register")]
pub async fn register_get(auth: Authenticated) -> Option<Template> {
    if !auth.user.is_admin {
        return Some(Template::render("errors/admin_only", context! {}));
    }
    Some(Template::render("admin/register", context! {}))
}
