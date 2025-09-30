use crate::auth::Authenticated;
use rocket::get;
use rocket_dyn_templates::{Template, context};

/// Route to show the admin a page to register new user.
#[get("/editusers")]
pub async fn edit_users(auth: Authenticated) -> Template {
    if !auth.user.is_admin {
        return Template::render("errors/admin_only", context! {});
    }
    Template::render("admin/users_list", context! {})
}
