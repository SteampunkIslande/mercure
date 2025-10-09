use crate::auth::Authenticated;
use rocket::{State, get, serde::json::Json};
use sqlx::SqlitePool;

use crate::{models::HgRun, routes::ApiResponse};

#[get("/listruns?<page>&<page_size>")]
pub async fn list_runs(
    pool: &State<SqlitePool>,
    authenticated: Authenticated,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Json<ApiResponse<Vec<HgRun>>> {
    if let Ok(runs_list) = HgRun::list_runs(pool, page, page_size).await {
        if runs_list.is_empty() {
            return Json(ApiResponse::error("Aucun run trouvé.".to_string()));
        }
        return Json(ApiResponse::success(runs_list));
    } else {
        return Json(ApiResponse::error(
            "Erreur lors de la récupération des runs.".to_string(),
        ));
    }
}
