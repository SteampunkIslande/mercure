use rocket::FromForm;
use rocket::get;
use rocket_dyn_templates::{Template, context};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, FromForm)]
pub struct RedirectForm {
    pub message: String,
    pub title: String,
    pub target_url: String,
    // Optionnel : permet de surcharger le délai en secondes
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
            title: title,
            message: message,
            target_url: target_url,
            seconds: seconds,
        },
    )
}
