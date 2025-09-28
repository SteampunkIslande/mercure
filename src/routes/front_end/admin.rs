use crate::auth::Authenticated;
use rocket::get;
use rocket_dyn_templates::{Template, context};

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
