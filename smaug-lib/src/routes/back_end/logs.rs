use std::collections::HashSet;
use std::path::PathBuf;

use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::{State, get};
use sqlx::SqlitePool;

use crate::auth::Authenticated;
use crate::config::get_smaug_config;
use crate::models::{Group, Run};

/// Serves the stdout/stderr logs of an attempt to users allowed to see the run.
///
/// Only `run-<run_id>-tentative-<attempt_number>.{stdout,stderr}` files are
/// exposed, so the route cannot be used to browse the whole logs directory.
#[get("/<filename>")]
pub async fn serve_run_log(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    filename: String,
) -> Result<NamedFile, Status> {
    let stem = filename
        .strip_suffix(".stdout")
        .or_else(|| filename.strip_suffix(".stderr"))
        .ok_or(Status::NotFound)?;

    if stem.contains('/') || stem.contains('\\') {
        return Err(Status::NotFound);
    }

    let (run_id, attempt_number) = parse_attempt_log_stem(stem).ok_or(Status::NotFound)?;

    let run = Run::get_run_from_id(run_id, pool)
        .await
        .map_err(|_| Status::NotFound)?;

    if !can_user_see_run(&auth, &run, pool).await? {
        return Err(Status::Forbidden);
    }

    let logs_dir = PathBuf::from(&get_smaug_config().logs_dir)
        .canonicalize()
        .map_err(|_| Status::NotFound)?;
    let file_path = logs_dir
        .join(&filename)
        .canonicalize()
        .map_err(|_| Status::NotFound)?;

    if !file_path.starts_with(&logs_dir) || !file_path.is_file() {
        return Err(Status::NotFound);
    }

    // The attempt must exist for this run; this also prevents serving leftovers
    // from an unrelated file that happens to follow the naming convention.
    crate::models::Attempt::get_attempt_from_number(attempt_number, run_id, pool)
        .await
        .map_err(|_| Status::NotFound)?;

    NamedFile::open(file_path)
        .await
        .map_err(|_| Status::NotFound)
}

/// Parses `run-<run_id>-tentative-<attempt_number>`.
fn parse_attempt_log_stem(stem: &str) -> Option<(i64, u32)> {
    let rest = stem.strip_prefix("run-")?;
    let (run_id, rest) = rest.split_once("-tentative-")?;
    if rest.contains('-') {
        return None;
    }
    Some((run_id.parse().ok()?, rest.parse().ok()?))
}

pub(crate) async fn can_user_see_run(
    auth: &Authenticated,
    run: &Run,
    pool: &SqlitePool,
) -> Result<bool, Status> {
    if auth.user.is_admin {
        return Ok(true);
    }

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

    Ok(!intersection::hash_set::intersection([current_user_groups, run_user_groups]).is_empty())
}

#[cfg(test)]
mod tests {
    use super::parse_attempt_log_stem;

    #[test]
    fn parses_attempt_log_names() {
        assert_eq!(
            parse_attempt_log_stem("run-00000007-tentative-00000003"),
            Some((7, 3))
        );
        assert_eq!(parse_attempt_log_stem("run-7"), None);
        assert_eq!(parse_attempt_log_stem("something-else"), None);
    }
}
