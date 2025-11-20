use std::str::FromStr;

use crate::models::RunStatus;
use crate::utils::format_french_date;
use crate::{auth::Authenticated, models::User, routes::ApiResponse};
use rocket::serde::json::Json;
use rocket::{State, get};
use serde_json::{Value, json};
use sqlx::Row;
use sqlx::SqlitePool;

fn create_header() -> Vec<Value> {
    vec![
        json!({"content": "Nom du run", "class": "content-column"}),
        json!({"content": "Utilisateur", "class": "content-column"}),
        json!({"content": "Date du run", "class": "content-column"}),
        json!({"content": "Statut", "class": "badge-column"}),
        json!({"content": "Tentative", "class": "numeric-column"}),
    ]
}

fn create_run_row(
    run_id: i64,
    run_name: &str,
    user_name: &str,
    run_date: &str,
    status: RunStatus,
    attempt_count: i64,
) -> Value {
    json!(vec![
        // Colonne 1 - Nom du run
        json!({
            "content": run_name,
            "href": Some(format!("/mercure/show/run/{}", run_id)),
            "td_class": "content-column"
        }),
        // Colonne 2 - Utilisateur
        json!({
            "content": user_name,
            "td_class": "badge-column"
        }),
        // Colonne 3 - Date du run
        json!({
            "content": format_french_date(run_date),
            "td_class": "badge-column"
        }),
        // Colonne 4 - Statut
        json!({
            "content": match status {
                RunStatus::Idle => "A valider",
                RunStatus::Failure(_) => "Echec",
                RunStatus::Pending => "En attente",
                RunStatus::Running => "Analyses en cours",
                RunStatus::Success => "Succès"
            },
            "class": format!("run-status {}", match status {
                RunStatus::Idle => "idle",
                RunStatus::Failure(_) => "failure",
                RunStatus::Pending => "pending",
                RunStatus::Running => "running",
                RunStatus::Success => "success"
            }),
            "td_class": "badge-column"
        }),
        // Colonne 5 - Tentatives
        json!({
            "content": attempt_count.to_string(),
            "td_class": "numeric-column"
        })
    ])
}

/// List runs regarding specific user
///
/// # Arguments
///
/// - `pool` (`&SqlitePool`) - The sqlite database connection
/// - `user` (`Option<&User>`) - The user to list runs of. `None` means no filter will be applied regarding user. Use for admin users.
/// - `page_size` (`Option<i64>`) - How many runs should be returned per page (default: 20)
/// - `page` (`Option<i64>`) - Page to show, starting at 1 (default: 1)
/// - `status` (`Option<String>`) - An optional string value to filter run status on. Final filter will be `LIKE '{status}%', meaning it will filter by this prefix`
///
/// # Returns
///
/// - `Result<(Vec<serde_json::Value>, i64), sqlx::Error>` - A `Vec<serde_json::Value>` (empty means that the query returned nothing).
///
/// # Errors
///
/// This function should not return any error, if it did, it would be from a sql syntax error or if the database is not accessible.
async fn list_runs(
    pool: &SqlitePool,
    user: Option<&User>,
    page_size: Option<i64>,
    page: Option<i64>,
    status: Option<String>,
) -> Result<(Vec<serde_json::Value>, i64), sqlx::Error> {
    // Here, if status is empty, this will match everything since status is a string type field and `LIKE %` will match any string!
    let status = status.unwrap_or_default();

    let (count_query, base_query) = match user {
        Some(u) => {
            let count_query = format!(
                r#"
                SELECT COUNT(r.run_id) as total
                FROM Runs r
                WHERE r.form_id IN (
                    SELECT f.form_id
                    FROM Formdef f
                    INNER JOIN FormdefHasGroup fg ON f.form_id = fg.form_id
                    INNER JOIN GroupHasUser gu ON fg.group_id = gu.group_id
                    WHERE gu.user_id = {} 
                )
                AND r.status LIKE '{status}%'
                "#,
                u.id
            );

            let base_query = format!(
                r#"
                SELECT r.run_id, r.run_name, r.attempt_count, r.status, r.run_date, u.username
                FROM Runs r
                INNER JOIN Users u ON r.user_id = u.id
                WHERE r.form_id IN (
                    SELECT f.form_id
                    FROM Formdef f
                    INNER JOIN FormdefHasGroup fg ON f.form_id = fg.form_id
                    INNER JOIN GroupHasUser gu ON fg.group_id = gu.group_id
                    WHERE gu.user_id = {} 
                )
                AND r.status LIKE '{status}%'
                ORDER BY r.run_date DESC
                LIMIT ? OFFSET ?
                "#,
                u.id
            );

            (count_query, base_query)
        }
        None => {
            let count_query = format!(
                "SELECT COUNT(r.run_id) as total FROM Runs r WHERE r.status LIKE '{status}%'"
            );

            let base_query = format!(
                r#"
                SELECT r.run_id, r.run_name, r.attempt_count, r.status, r.run_date, u.username
                FROM Runs r
                INNER JOIN Users u ON r.user_id = u.id
                WHERE r.status LIKE '{status}%'
                ORDER BY r.run_date DESC
                LIMIT ? OFFSET ?
            "#
            );

            (count_query, base_query)
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
            let username: String = row.try_get("username").ok()?;
            let run_date: String = row.try_get("run_date").ok()?;
            let attempt_count: i64 = row.try_get("attempt_count").ok()?;
            let status_str: String = row.try_get("status").ok()?;
            let status: RunStatus = RunStatus::from_str(&status_str).ok()?;

            Some(create_run_row(
                run_id,
                &run_name,
                &username,
                &run_date,
                status,
                attempt_count,
            ))
        })
        .collect();

    Ok((runs_data, total_count))
}

#[get("/listruns?<page>&<page_size>&<status>")]
pub async fn list_runs_get(
    pool: &State<SqlitePool>,
    authenticated: Authenticated,
    page: Option<i64>,
    page_size: Option<i64>,
    status: Option<String>,
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
        status,
    )
    .await
    {
        Ok((runs_list, total_count)) => {
            let page_size_val = page_size.unwrap_or(5);
            let current_page = page.unwrap_or(1);
            let total_pages = (total_count + page_size_val - 1) / page_size_val;

            let title = if authenticated.user.is_admin {
                "Liste des runs (tous les groupes)"
            } else {
                "Liste des runs de vos groupes"
            };

            return Json(ApiResponse::success(json!({
                    "title": title,
                    "header": create_header(),
                    "table": runs_list,
                    "pagination": {
                        "current_page": current_page,
                        "total_pages": total_pages,
                        "page_size": page_size_val,
                        "total_count": total_count
                    }
            })));
        }
        Err(_e) => {
            return Json(ApiResponse::error(
                "Erreur lors de la récupération des runs.".to_string(),
            ));
        }
    }
}
