use crate::{auth::Authenticated, models::Group};
use rocket::{State, get};
use rocket_dyn_templates::{Template, context};
use sqlx::SqlitePool;

#[get("/home")]
pub async fn home_get(auth: Authenticated, pool: &State<SqlitePool>) -> Template {
    let is_admin = auth.user.is_admin;
    match Group::get_user_groups(pool, auth.user.id).await {
        Ok(user_groups) => Template::render(
            "common/home",
            context! {user:auth.user,user_groups,is_admin},
        ),
        Err(e) => Template::render(
            "errors/error",
            context! {error:e.to_string(),message:"Erreur lors de l'obtention de vos groupes",title:"Erreur dans la base de données",back_to:["/mercure"]},
        ),
    }
}
