use crate::auth::Authenticated;
use rocket::get;
use rocket_dyn_templates::{Template, context};

#[get("/forms/updates")]
pub async fn forms_list_updates_get(auth: Authenticated) -> Template {
    if !auth.user.is_admin {
        Template::render("errors/admin_only", context! {user_name:auth.user.username})
    } else {
        Template::render("admin/formupdatemanager", context! {})
    }
}
