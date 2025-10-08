use crate::models::{HgRun, RunStatus};

use super::ModelError;
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalysisStateMachineError {
    #[error("Invalid translation from {} to {}",.from,.to)]
    InvalidTransition { from: String, to: String },
    #[error(transparent)]
    ModelError(#[from] ModelError),
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
    super::hgrun::HgRun::complete_failure(run_id, reason, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
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
