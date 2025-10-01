use crate::{auth::Authenticated, models::User};
use rocket::{State, get};
use rocket_dyn_templates::{Template, context};
use sqlx::SqlitePool;

#[get("/dashboard")]
pub async fn admin_dashboard_get(auth: Authenticated) -> Option<Template> {
    if !auth.user.is_admin {
        return None;
    }
    Some(Template::render(
        "admin/dashboard",
        context! {user:auth.user},
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
        return None;
    }
    let edited_user = User::find_by_id(user_id, pool).await.ok()?;
    Some(Template::render(
        "admin/passedit",
        context! {auth_user: auth.user, edited_user:edited_user},
    ))
}

/// Route to show the admin a page to register new user.
#[get("/register")]
pub async fn register_get(auth: Authenticated) -> Option<rocket::fs::NamedFile> {
    if !auth.user.is_admin {
        return None;
    }
    rocket::fs::NamedFile::open("static/admin/register.html")
        .await
        .ok()
}
