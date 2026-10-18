use crate::{
    models::{Attempt, FormIdentification, Run, RunDefinitionError, RunStatus},
    pipeline_exec::{GitCheckError, versionning},
};

use super::ModelError;
use sqlx::SqlitePool;
use wherror::Error;

#[derive(Error, Debug)]
pub enum AnalysisStateMachineError {
    #[error("Transition invalide de {} vers {}",.from,.to)]
    InvalidTransition { from: String, to: String },
    #[error("{location}: {source}")]
    ModelError {
        #[from]
        source: ModelError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{location}: {source}")]
    RunDefinitionError {
        #[from]
        source: RunDefinitionError,
        location: &'static std::panic::Location<'static>,
    },
    #[error("{0}")]
    InvalidOperation(String),
    #[error("{location}: {source}")]
    GitCheckError {
        #[from]
        source: GitCheckError,
        location: &'static std::panic::Location<'static>,
    },
}

/// Valide le formulaire pour ce run : Idle -> Pending, crée une nouvelle tentative (appelé par le backend)
pub async fn validate_form(
    run_id: i64,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let run: Run = Run::get_run_from_id(run_id, pool).await?;

    if !matches!(run.status, RunStatus::Idle) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Pending.to_string(),
        });
    }

    let current_commit_hash = match run.form_identification {
        FormIdentification::GitBranchAndPath { branch, .. } => {
            versionning::get_latest_commit(Some(&branch))
                .await?
                .into_iter()
                .next()
                .ok_or(GitCheckError::NoSuchBranch(branch))?
                .last_commit
        }
        FormIdentification::GitCommit { commit_hash, .. } => commit_hash,
        FormIdentification::None => "".to_string(),
    };
    super::hgrun::Run::validate_form(run_id, pool).await?;
    super::attempt::Attempt::new_attempt(run_id, &current_commit_hash, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}

/// Démarre le run : Pending -> Running (appelé par la routine de vérification)
pub async fn start_run_analysis(
    run_id: i64,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let run: Run = Run::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Pending) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Running.to_string(),
        });
    }
    super::hgrun::Run::start_run(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)?;
    super::attempt::Attempt::start_run(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}

/// Termine le run avec succès : Running -> Success (appelé par la routine de vérification)
pub async fn complete_success(
    run_id: i64,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let run: Run = Run::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Running) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Success.to_string(),
        });
    }
    super::hgrun::Run::complete_success(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)?;
    super::attempt::Attempt::complete_success(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}

/// Termine le run avec échec : Running -> Failure (appelé par la routine de vérification)
pub async fn complete_failure(
    run_id: i64,
    reason: &str,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let reason = reason.to_string();
    let run: Run = Run::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Running) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Failure(reason).to_string(),
        });
    }
    super::hgrun::Run::complete_failure(run_id, &reason, pool)
        .await
        .map_err(AnalysisStateMachineError::from)?;
    super::attempt::Attempt::complete_failure(run_id, &reason, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}

/// Transitionne un run en échec si le run est introuvable, ou toute ature erreur empêchant le lancement : Pending -> Failure (appelé par la routine de vérification)
pub async fn fail_cannot_analyse_run(
    run_id: i64,
    reason: &str,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let run = Run::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Pending) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Failure(reason.to_string()).to_string(),
        });
    }
    super::hgrun::Run::complete_failure(run_id, reason, pool).await?;
    super::attempt::Attempt::complete_failure(run_id, reason, pool).await?;
    Ok(())
}

/// Commenter une tentative
pub async fn comment_attempt(
    run_id: i64,
    attempt_number: u32,
    comment: &str,
    pool: &SqlitePool,
) -> Result<(), AnalysisStateMachineError> {
    let attempt: Attempt = Attempt::get_attempt_from_number(attempt_number, run_id, pool).await?;
    if !matches!(attempt.status, RunStatus::Success | RunStatus::Failure(_)) {
        return Err(AnalysisStateMachineError::InvalidOperation(
            "Vous ne pouvez commenter que des tentatives terminées".to_string(),
        ));
    }
    Attempt::update_comment(run_id, attempt_number, comment, pool).await?;
    Ok(())
}

/// Relance le run : Success/Failure -> Idle
pub async fn relaunch_run(run_id: i64, pool: &SqlitePool) -> Result<(), AnalysisStateMachineError> {
    let run: Run = Run::get_run_from_id(run_id, pool).await?;
    if !matches!(run.status, RunStatus::Success | RunStatus::Failure(_)) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Idle.to_string(),
        });
    }
    super::hgrun::Run::relaunch_run(run_id, pool)
        .await
        .map_err(AnalysisStateMachineError::from)
}
