use rocket::fs::NamedFile;
use rocket::get;
use rocket::http::{Cookie, CookieJar};
use rocket::serde::json::Json;

use super::ApiResponse;

#[get("/")]
pub async fn welcome_page_get() -> Option<NamedFile> {
    NamedFile::open("static/common/welcome.html").await.ok()
}

#[get("/login")]
pub async fn login_get() -> Option<NamedFile> {
    NamedFile::open("static/common/login.html").await.ok()
}

#[get("/logout")]
pub fn logout_get(cookies: &CookieJar<'_>) -> Json<ApiResponse<()>> {
    cookies.remove_private(Cookie::build("user_id"));
    Json(ApiResponse::success(()))
}
