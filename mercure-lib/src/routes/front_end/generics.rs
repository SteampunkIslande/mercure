use rocket::get;
use rocket_dyn_templates::{Template, context};

#[get("/")]
pub async fn welcome_page_get() -> Option<Template> {
    Some(Template::render("common/welcome", context! {}))
}

#[get("/login")]
pub async fn login_get() -> Option<Template> {
    Some(Template::render("common/login", context! {}))
}
