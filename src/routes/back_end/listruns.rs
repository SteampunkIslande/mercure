use std::str::FromStr;

use crate::models::RunStatus;
use crate::{auth::Authenticated, models::User, routes::ApiResponse};
use rocket::serde::json::Json;
use rocket::{State, get};
use serde_json::{Value, json};
use sqlx::Row;
use sqlx::SqlitePool;

async fn list_runs(
    pool: &SqlitePool,
    user: Option<&User>,
    page_size: Option<i64>,
    page: Option<i64>,
) -> Result<(Vec<serde_json::Value>, i64), sqlx::Error> {
    let count_query = match user {
        Some(u) => {
            format!(
                r#"
                SELECT COUNT(DISTINCT r.run_id) as total
                FROM Runs r
                INNER JOIN Formdef f ON r.form_id = f.form_id
                INNER JOIN FormdefHasGroup fg ON f.form_id = fg.form_id
                INNER JOIN GroupHasUser gu ON fg.group_id = gu.group_id
                WHERE gu.user_id = {}
            "#,
                u.id
            )
        }
        None => {
            format!(
                r#"
                SELECT COUNT(DISTINCT r.run_id) as total
                FROM Runs r
                INNER JOIN Formdef f ON r.form_id = f.form_id
                INNER JOIN FormdefHasGroup fg ON f.form_id = fg.form_id
                INNER JOIN GroupHasUser gu ON fg.group_id = gu.group_id
                "#
            )
        }
    };

    let base_query = match user {
        Some(u) => {
            format!(
                r#"
                SELECT DISTINCT r.run_id, r.run_name, r.attempt_count, r.status
                FROM Runs r
                INNER JOIN Formdef f ON r.form_id = f.form_id
                INNER JOIN FormdefHasGroup fg ON f.form_id = fg.form_id
                INNER JOIN GroupHasUser gu ON fg.group_id = gu.group_id
                WHERE gu.user_id = {}
                ORDER BY r.creation_date DESC
                LIMIT ? OFFSET ?
            "#,
                u.id
            )
        }
        None => {
            format!(
                r#"
                SELECT DISTINCT r.run_id, r.run_name, r.attempt_count, r.status
                FROM Runs r
                INNER JOIN Formdef f ON r.form_id = f.form_id
                INNER JOIN FormdefHasGroup fg ON f.form_id = fg.form_id
                INNER JOIN GroupHasUser gu ON fg.group_id = gu.group_id
                ORDER BY r.creation_date DESC
                LIMIT ? OFFSET ?
                "#
            )
        }
    };

    // Get total count
    let total_count: i64 = sqlx::query(&count_query)
        .fetch_one(pool)
        .await?
        .try_get("total")?;

    let runs_data = sqlx::query(&base_query)
        .bind(page_size.unwrap_or(20))
        .bind((page.unwrap_or(1) - 1) * page_size.unwrap_or(20))
        .fetch_all(pool)
        .await?
        .iter()
        .filter_map(|row| {
            let run_id: i64 = row.try_get("run_id").ok()?;
            let run_name: String = row.try_get("run_name").ok()?;
            let attempt_count: i64 = row.try_get("attempt_count").ok()?;
            let status_str: String = row.try_get("status").ok()?;
            let status: RunStatus = RunStatus::from_str(&status_str).ok()?;

            Some(json!(vec![
                json!({
                    "content":run_name,
                    "href":Some(format!("/mercure/show/run/{run_id}"))
                }),
                json!({
                    "content":status_str,
                    "class":format!("run-status {}",match status
                {
                    RunStatus::Idle=>"idle",
                    RunStatus::Failure(_)=>"failure",
                    RunStatus::Pending=>"pending",
                    RunStatus::Running=>"running",
                    RunStatus::Success=>"success"
                })}),
                json!({"content":attempt_count.to_string()})
            ]))
        })
        .collect();

    Ok((runs_data, total_count))
}

#[get("/listruns?<page>&<page_size>")]
pub async fn list_runs_get(
    pool: &State<SqlitePool>,
    authenticated: Authenticated,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Json<ApiResponse<Value>> {
    match list_runs(
        pool,
        if !authenticated.user.is_admin {
            Some(&authenticated.user)
        } else {
            None
        },
        page_size,
        page,
    )
    .await
    {
        Ok((runs_list, total_count)) => {
            if runs_list.is_empty() {
                return Json(ApiResponse::error("Aucun run trouvé.".to_string()));
            } else {
                let page_size_val = page_size.unwrap_or(5);
                let current_page = page.unwrap_or(1);
                let total_pages = (total_count + page_size_val - 1) / page_size_val;

                return Json(ApiResponse::success(json!({
                        "title": "Liste des runs de vos groupes",
                        "header": vec!["Nom du run","Statut","Tentative"] ,
                        "table": runs_list,
                        "pagination": {
                            "current_page": current_page,
                            "total_pages": total_pages,
                            "page_size": page_size_val,
                            "total_count": total_count
                        }
                })));
            }
        }
        Err(_e) => {
            return Json(ApiResponse::error(
                "Erreur lors de la récupération des runs.".to_string(),
            ));
        }
    }
}
