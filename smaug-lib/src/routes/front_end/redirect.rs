use crate::templates::{Template, context};
use rocket::FromForm;
use rocket::get;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, FromForm)]
pub struct RedirectForm {
    pub message: String,
    pub title: String,
    pub target_url: String,
    // Optional: allows overriding the delay in seconds
    pub seconds: Option<u64>,
}

#[get("/redirect?<message>&<title>&<target_url>&<seconds>")]
pub async fn redirect_get(
    message: String,
    title: String,
    target_url: String,
    seconds: Option<i64>,
) -> Template {
    let seconds = seconds.unwrap_or(5);
    Template::render(
        "common/redirect",
        context! {
            title=> title,
            message=> message,
            target_url=> target_url,
            seconds=> seconds,
        },
    )
}
