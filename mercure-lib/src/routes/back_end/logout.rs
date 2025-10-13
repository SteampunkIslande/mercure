use rocket::get;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::uri;

#[get("/logout")]
pub fn logout_get(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::build("user_id"));
    Redirect::to(uri!("/mercure"))
}
