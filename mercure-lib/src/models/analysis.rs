use crate::models::{HgAttempt, HgRun, RunStatus};

use super::ModelError;
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalysisStateMachineError {
    #[error("Invalid transition from {} to {}",.from,.to)]
    InvalidTransition { from: String, to: String },
    #[error(transparent)]
    ModelError(#[from] ModelError),
    #[error("{0}")]
    InvalidOperation(String),
}

/// Valide le formulaire pour ce run : Idle -> Pending, crée une nouvelle tentative
pub async fn validate_form(
    run_id: i64,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let run: HgRun = HgRun::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Idle) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Pending.to_string(),
        });
    }
    super::hgrun::HgRun::validate_form(run_id, pool).await?;
    super::attempt::HgAttempt::new_attempt(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}

/// Démarre le run : Pending -> Running (appelé par la routine de vérification)
pub async fn start_run_analysis(
    run_id: i64,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let run: HgRun = HgRun::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Pending) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Running.to_string(),
        });
    }
    super::hgrun::HgRun::start_run(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)?;
    super::attempt::HgAttempt::start_run(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}

/// Termine le run avec succès : Running -> Success (appelé par la routine de vérification)
pub async fn complete_success(
    run_id: i64,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let run: HgRun = HgRun::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Running) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Success.to_string(),
        });
    }
    super::hgrun::HgRun::complete_success(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)?;
    super::attempt::HgAttempt::complete_success(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}

/// Termine le run avec échec : Running -> Failure (appelé par la routine de vérification)
pub async fn complete_failure(
    run_id: i64,
    reason: String,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let run: HgRun = HgRun::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Running) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Failure(reason).to_string(),
        });
    }
    super::hgrun::HgRun::complete_failure(run_id, &reason, pool)
        .await
        .map_err(AnalysisStateMachineError::from)?;
    super::attempt::HgAttempt::complete_failure(run_id, &reason, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}

/// Transitionne un run en échec si le run est introuvable, ou toute ature erreur empêchant le lancement : Pending -> Failure
pub async fn fail_cannot_analyse_run(
    run_id: i64,
    reason: &str,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let run = HgRun::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Pending) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Failure(reason.to_string()).to_string(),
        });
    }
    super::hgrun::HgRun::complete_failure(run_id, reason, pool).await?;
    super::attempt::HgAttempt::complete_failure(run_id, reason, pool).await?;
    Ok(())
}

/// Commenter une tentative
pub async fn comment_attempt(
    run_id: i64,
    attempt_number: i64,
    comment: &str,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let attempt: HgAttempt =
        HgAttempt::get_attempt_from_number(attempt_number, run_id, pool).await?;
    if !matches!(attempt.status, RunStatus::Success | RunStatus::Failure(_)) {
        return Err(AnalysisStateMachineError::InvalidOperation(
            "You can only comment on finished attempts".to_string(),
        ));
    }
    HgAttempt::update_comment(run_id, attempt_number, comment, pool).await?;
    Ok(())
}

/// Relance le run : Success/Failure -> Idle
pub async fn relaunch_run(run_id: i64, pool: &SqlitePool) -> Result<(), AnalysisStateMachineError> {
    let run: HgRun = HgRun::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Success | RunStatus::Failure(_)) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Idle.to_string(),
        });
    }
    super::hgrun::HgRun::relaunch_run(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}
