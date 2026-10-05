use crate::models::Form;
use crate::routes::frontend::FrontendError;
use crate::templates::{Template, context};
use rocket::{State, get};
use sqlx::SqlitePool;

use crate::auth::Authenticated;

#[get("/runs/submit?<branch>&<form_path>")]
pub async fn new_run_get(
    auth: Authenticated,
    form_path: &str,
    branch: &str,
    _pool: &State<SqlitePool>,
) -> Result<Template, FrontendError> {
    let form = Form::get_form(branch, form_path).await?;
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
