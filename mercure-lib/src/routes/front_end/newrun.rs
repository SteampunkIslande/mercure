use crate::models::{Form, FormIdentification};
use crate::routes::frontend::FrontendError;
use crate::templates::{Template, context};
use rocket::{State, get};
use sqlx::SqlitePool;

use crate::auth::Authenticated;

#[get("/runs/submit/branch/<branch>/<file_path>")]
pub async fn new_run_get(
    auth: Authenticated,
    branch: String,
    file_path: String,
    _pool: &State<SqlitePool>,
) -> Result<Template, FrontendError> {
    let form = Form::get_form(FormIdentification::GitBranchAndPath { branch, file_path }).await?;
    let variables: Vec<String> = form.variables.iter().map(|v| v.to_html_safe()).collect();
    Ok(Template::render(
        "common/editrun",
        context! {
            form=> form,
            user_defined_vars=> &variables,
            user=>&auth.user
        },
    ))
}
