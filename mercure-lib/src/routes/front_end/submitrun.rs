use crate::templates::{Template, context};
use rocket::{State, get};
use sqlx::SqlitePool;

use crate::auth::Authenticated;
use crate::config::get_mercure_config;
use std::path::PathBuf;

#[get("/runs/submit/<branch>/<form_path..>")]
pub async fn new_run_get(
    auth: Authenticated,
    form_path: PathBuf,
    branch: String,
    pool: &State<SqlitePool>,
) -> Template {
    let config = get_mercure_config();
    Template::render("common/editrun", context! {})
}
