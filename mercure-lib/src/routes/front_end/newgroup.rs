use rocket::get;

use crate::routes::frontend::FrontendError;
use crate::templates::{Template, context};

use crate::auth::Authenticated;

#[get("/groups/new")]
pub async fn new_group(auth: Authenticated) -> Result<Template, FrontendError> {
    if !auth.user.is_admin {
        Ok(Template::render("errors/admin_only", context! {}))
    } else {
        Ok(Template::render("admin/new_group", context! {}))
    }
}
