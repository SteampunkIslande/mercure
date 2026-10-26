use crate::auth::{AuthError, Authenticated};
use crate::models::{Group, Run, RunStatus, User};
use crate::routes::ApiResponse;
use anyhow::Context;
use rocket::serde::json::Json;
use rocket::{State, get};
use serde_json::Value;
use sqlx::SqlitePool;

/// Résout ce que l'utilisateur courant est autorisé à voir :
///
/// - un admin voit tous les runs (`groups` = None) ;
/// - un utilisateur standard ne voit que les runs des formulaires associés à au
///   moins un de ses groupes (voir `Run::fetch_runs_table`).
///
/// Renvoie `None` pour un admin (aucun filtre), les groupes de l'utilisateur sinon.
async fn resolve_viewer(pool: &SqlitePool, user: &User) -> anyhow::Result<Option<Vec<Group>>> {
    if user.is_admin {
        Ok(None)
    } else {
        let groups = Group::get_user_groups(pool, user.id)
            .await
            .with_context(|| {
                format!(
                    "Impossible de lire les groupes de l'utilisateur {}",
                    user.id
                )
            })?;
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
) -> Result<Json<ApiResponse<Value>>, AuthError> {
    let groups = resolve_viewer(pool, &authenticated.user).await?;
    let payload = Run::list_runs(pool, page_size, page, status, groups).await?;
    Ok(Json(ApiResponse::success(payload)))
}

#[get("/searchrun?<page>&<page_size>&<status>&<date_from>&<date_to>&<run_name_search>")]
#[allow(clippy::too_many_arguments)]
pub async fn search_run_get(
    pool: &State<SqlitePool>,
    authenticated: Authenticated,
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    run_name_search: Option<String>,
) -> Result<Json<ApiResponse<Value>>, AuthError> {
    let groups = resolve_viewer(pool, &authenticated.user).await?;
    let payload = Run::search_runs(
        pool,
        page_size,
        page,
        status.as_deref(),
        date_from.as_deref(),
        date_to.as_deref(),
        run_name_search.as_deref(),
        groups,
    )
    .await?;
    Ok(Json(ApiResponse::success(payload)))
}
