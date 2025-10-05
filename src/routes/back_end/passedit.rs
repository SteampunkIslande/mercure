use rocket::{State, post, serde::json::Json};
use sqlx::SqlitePool;

use crate::auth::Authenticated;
use crate::models::User;
use crate::{models::PasswordUpdate, routes::ApiResponse};

#[post("/passedit", format = "application/json", data = "<data>")]
pub async fn password_edit_post(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    data: Json<PasswordUpdate<'_>>,
) -> Json<ApiResponse<String>> {
    if auth.user.id != data.0.user_id && !auth.user.is_admin {
        return Json(ApiResponse::error("Seuls les administrateurs peuvent modifier le mot de passe d'un autre utilisateur".to_string()));
    }
    match User::update_password(data.0, pool).await {
        Ok(()) => Json(ApiResponse::success(String::from(
            "Mot de passe mis à jour avec succès",
        ))),
        Err(e) => Json(ApiResponse::error(format!("{}", e))),
    }
}
