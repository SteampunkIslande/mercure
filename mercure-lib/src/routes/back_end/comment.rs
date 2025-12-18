use crate::auth::Authenticated;
use crate::models::HgRun;
use crate::models::analysis::comment_attempt;
use crate::routes::ApiResponse;
use rocket::State;
use rocket::post;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCommentForm {
    run_id: i64,
    attempt_number: i64,
    comment: String,
}

#[post("/comment", data = "<form>")]
pub async fn update_comment(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    form: Json<UpdateCommentForm>,
) -> Json<ApiResponse<Value>> {
    let payload = form.0;
    if let Ok(run) = HgRun::get_run_from_id(payload.run_id, pool).await {
        if auth.user.is_admin || auth.user.id == run.user.id {
            match comment_attempt(
                payload.run_id,
                payload.attempt_number,
                &payload.comment,
                pool,
            )
            .await
            {
                Ok(_) => Json(ApiResponse::success(
                    json!({"message":"Commentaire mis à jour avec succès!"}),
                )),
                Err(e) => Json(ApiResponse::error(format!(
                    "Erreur lors de la mise à jour du commentaire: {}",
                    e
                ))),
            }
        } else {
            Json(ApiResponse::error("Permission refusée.".to_string()))
        }
    } else {
        Json(ApiResponse::error("Tentative non trouvée.".to_string()))
    }
}
