use crate::routes::frontend::FrontendError;
use crate::templates::{Template, context};
use crate::{auth::Authenticated, models::Group};
use anyhow::Context;
use rocket::{State, get};
use sqlx::SqlitePool;

#[get("/home")]
pub async fn home_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
) -> Result<Template, FrontendError> {
    let is_admin = auth.user.is_admin;
    let user_groups = Group::get_user_groups(pool, auth.user.id)
        .await
        .context("Erreur lors de la récupération de vos groupes")?;
    Ok(Template::render(
        "common/home",
        context! {user=>auth.user,user_groups,is_admin},
    ))
}
