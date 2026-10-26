use crate::models::{FileIdentification, Form};
use crate::routes::frontend::FrontendError;
use crate::templates::{Template, context};
use rocket::http::uri::{Segments, fmt::Path};
use rocket::{State, get};
use sqlx::SqlitePool;

use crate::auth::Authenticated;

#[get("/runs/submit/branch/<branch>/<file_path..>")]
pub async fn new_run_get(
    auth: Authenticated,
    branch: &str,
    file_path: Segments<'_, Path>,
    _pool: &State<SqlitePool>,
) -> Result<Template, FrontendError> {
    let form = Form::get_form(FileIdentification::GitBranchAndPath {
        branch: branch.to_string(),
        file_path: file_path
            .to_path_buf(true)
            .map_err(|e| anyhow::anyhow!("Chemin de formulaire invalide: {e:?}"))?,
    })
    .await?;
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
