use rocket::State;
use rocket::get;
use sqlx::SqlitePool;

use rocket_dyn_templates::{Template, context};

use crate::auth::Authenticated;

#[get("/show/run/<run_id>")]
pub async fn show_run_get(auth: Authenticated, pool: &State<SqlitePool>, run_id: i64) -> Template {
    Template::render("common/run", context! {})
}

#[get("/show/runs")]
pub async fn list_runs() -> Template {
    Template::render("common/listruns", context! {})
}
