use rocket::get;
use rocket_dyn_templates::{Template, context};

use crate::auth::Authenticated;

#[get("/")]
pub async fn welcome_page_get() -> Option<Template> {
    Some(Template::render("common/welcome", context! {}))
}

#[get("/login")]
pub async fn login_get() -> Option<Template> {
    Some(Template::render("common/login", context! {}))
}

#[get("/success?<origin>&<message>")]
pub fn success_page_get(authenticated: Authenticated, origin: String, message: String) -> Template {
    let home_uri = if authenticated.user.is_admin {
        "/mercure/admin/dashboard"
    } else {
        "/mercure/home"
    };
    Template::render(
        "common/success",
        context! {
            origin: &origin,
            message: &message,
            home: home_uri
        },
    )
}
