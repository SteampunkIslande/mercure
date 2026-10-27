use std::collections::HashSet;
use std::path::PathBuf;

use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::{State, get};
use sqlx::SqlitePool;

use crate::auth::Authenticated;
use crate::config::get_smaug_config;
use crate::models::{Group, Run};

/// Serves a file produced by a script in its `exports-<attempt_number>` folder.
///
/// The analysis directory is laid out as
/// `<analysis_dir>/run-<run_id>/exports-<attempt_number>/...`. Scripts may create
/// one subdirectory below `exports-<attempt_number>` (for example `logs/` or
/// `rapports/`), so paths of up to two components are accepted.
#[get("/<run_id>/<attempt_number>/<path..>")]
pub async fn serve_export_file(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    run_id: i64,
    attempt_number: u32,
    path: PathBuf,
) -> Result<NamedFile, Status> {
    let run = Run::get_run_from_id(run_id, pool)
        .await
        .map_err(|_| Status::NotFound)?;

    if !auth.user.is_admin {
        let current_user_groups: HashSet<i64> = Group::get_user_groups(pool, auth.user.id)
            .await
            .map_err(|_| Status::InternalServerError)?
            .iter()
            .map(|group| group.id)
            .collect();
        let run_user_groups: HashSet<i64> = Group::get_user_groups(pool, run.user_id)
            .await
            .map_err(|_| Status::InternalServerError)?
            .iter()
            .map(|group| group.id)
            .collect();
        if intersection::hash_set::intersection([current_user_groups, run_user_groups]).is_empty() {
            return Err(Status::Forbidden);
        }
    }

    // `exports-<attempt>/<subdir>/<file>` is the deepest supported layout.
    let depth = path.components().count();
    if depth == 0 || depth > 2 {
        return Err(Status::NotFound);
    }

    let analysis_dir = PathBuf::from(&get_smaug_config().analysis_dir);
    let export_dir = analysis_dir
        .join(format!("run-{:0>8}", run_id))
        .join(format!("exports-{}", attempt_number))
        .canonicalize()
        .map_err(|_| Status::NotFound)?;
    let file_path = export_dir
        .join(&path)
        .canonicalize()
        .map_err(|_| Status::NotFound)?;

    if !file_path.starts_with(&export_dir) || !file_path.is_file() {
        return Err(Status::NotFound);
    }

    NamedFile::open(file_path)
        .await
        .map_err(|_| Status::NotFound)
}
