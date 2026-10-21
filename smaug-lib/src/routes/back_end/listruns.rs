use crate::auth::Authenticated;
use crate::models::{Group, ModelError, Run, RunStatus, User};
use crate::routes::ApiResponse;
use rocket::serde::json::Json;
use rocket::{State, get};
use serde_json::Value;
use sqlx::SqlitePool;

/// Résout ce que l'utilisateur courant est autorisé à voir :
///
/// - un admin voit tous les runs (`groups` = None) ;
/// - un utilisateur standard ne voit que les runs des formulaires associés à au
///   moins un de ses groupes (voir `Run::fetch_runs_table`).
/// Renvoie soit None
async fn resolve_viewer(pool: &SqlitePool, user: &User) -> Result<Option<Vec<Group>>, ModelError> {
    if user.is_admin {
        Ok(None)
    } else {
        let groups = Group::get_user_groups(pool, user.id).await?;
        Ok(Some(groups))
    }
}

#[get("/listruns?<page>&<page_size>&<status>")]
pub async fn list_runs_get(
    pool: &State<SqlitePool>,
    authenticated: Authenticated,
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<RunStatus>,
) -> Json<ApiResponse<Value>> {
    let groups = match resolve_viewer(pool, &authenticated.user).await {
        Ok(groups) => groups,
        Err(_) => {
            return Json(ApiResponse::error(
                "Erreur lors de la récupération de vos groupes.".to_string(),
            ));
        }
    };

    match Run::list_runs(pool, page_size, page, status, groups).await {
        Ok(payload) => Json(ApiResponse::success(payload)),
        Err(_) => Json(ApiResponse::error(
            "Erreur lors de la récupération des runs.".to_string(),
        )),
    }
}

#[get("/searchrun?<page>&<page_size>&<status>&<date_from>&<date_to>&<run_name_search>")]
pub async fn search_run_get(
    pool: &State<SqlitePool>,
    authenticated: Authenticated,
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    run_name_search: Option<String>,
) -> Json<ApiResponse<Value>> {
    let groups = match resolve_viewer(pool, &authenticated.user).await {
        Ok(groups) => groups,
        Err(_) => {
            return Json(ApiResponse::error(
                "Erreur lors de la récupération de vos groupes.".to_string(),
            ));
        }
    };

    match Run::search_runs(
        pool,
        page_size,
        page,
        status.as_deref(),
        date_from.as_deref(),
        date_to.as_deref(),
        run_name_search.as_deref(),
        groups,
    )
    .await
    {
        Ok(payload) => Json(ApiResponse::success(payload)),
        Err(_) => Json(ApiResponse::error(
            "Erreur lors de la recherche des runs.".to_string(),
        )),
    }
}
