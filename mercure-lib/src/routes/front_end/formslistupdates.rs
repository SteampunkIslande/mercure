use crate::auth::Authenticated;
use crate::templates::{Template, context};
use rocket::get;

#[get("/forms/updates")]
pub async fn forms_list_updates_get(auth: Authenticated) -> Template {
    if !auth.user.is_admin {
        Template::render(
            "errors/admin_only",
            context! {user_name=>auth.user.username},
        )
    } else {
        Template::render("admin/formupdatemanager", context! {})
    }
}
