use rocket::fs::NamedFile;
use rocket::get;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::uri;
use rocket_dyn_templates::{Template, context};

use crate::auth::Authenticated;

#[get("/")]
pub async fn welcome_page_get() -> Option<NamedFile> {
    NamedFile::open("static/common/welcome.html").await.ok()
}

#[get("/login")]
pub async fn login_get() -> Option<NamedFile> {
    NamedFile::open("static/common/login.html").await.ok()
}

#[get("/logout")]
pub fn logout_get(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::build("user_id"));
    Redirect::to(uri!("/mercure"))
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
