use crate::{
    config::{SmaugConfig, get_smaug_config},
    models::{Attempt, FileIdentification, Run, RunStatus},
    pipeline_exec::versionning,
};

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use wherror::Error;

/// Errors of the run state machine.
///
/// The state machine functions return `anyhow::Result`; the errors below are the
/// ones callers may want to recognise (a refused transition is a *user* error,
/// not an internal failure).///
/// These types are the *named* errors of the layer; the functions themselves
/// return `anyhow::Result`. The `InternalError` variant only exists to give the
/// edge errors (FrontendError / AuthError) a single `#[from] anyhow::Error`
/// conversion point; it is not produced by the model layer itself.
#[derive(Error, Debug)]
pub enum AnalysisStateMachineError {
    #[error("Transition invalide de {} vers {}",.from,.to)]
    InvalidTransition { from: String, to: String },
    #[error("{0}")]
    InvalidOperation(String),
    #[error("{location}: {source}")]
    InternalError {
        #[from]
        source: anyhow::Error,
        location: &'static std::panic::Location<'static>,
    },
}

/// Valide le formulaire pour ce run : Idle -> Pending, crée une nouvelle tentative (appelé par le backend)
pub async fn validate_form(run_id: i64, pool: &SqlitePool) -> Result<()> {
    let SmaugConfig { pipelines, .. } = get_smaug_config();
    let run: Run = Run::get_run_from_id(run_id, pool)
        .await
        .with_context(|| format!("Impossible de relire le run {run_id}"))?;

    if !matches!(run.status, RunStatus::Idle) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Pending.to_string(),
        }
        .into());
    }

    let current_commit_hash = match run.form_identification {
        FileIdentification::GitBranchAndPath { branch, .. } => {
            versionning::get_latest_commit(&branch, &pipelines)
                .await
                .with_context(|| {
                    format!("Impossible de récupérer le dernier commit de la branche {branch}")
                })?
        }
        FileIdentification::GitCommit { commit_hash, .. } => commit_hash,
        FileIdentification::None => "".to_string(),
    };
    super::hgrun::Run::validate_form(run_id, pool)
        .await
        .with_context(|| format!("Impossible de valider le run {run_id}"))?;
    super::attempt::Attempt::new_attempt(run_id, &current_commit_hash, pool)
        .await
        .with_context(|| format!("Impossible de créer la tentative du run {run_id}"))
}

/// Démarre le run : Pending -> Running (appelé par la routine de vérification)
pub async fn start_run_analysis(run_id: i64, pool: &SqlitePool) -> Result<()> {
    let run: Run = Run::get_run_from_id(run_id, pool)
        .await
        .with_context(|| format!("Impossible de relire le run {run_id}"))?;
    if !matches!(run.status, RunStatus::Pending) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Running.to_string(),
        }
        .into());
    }
    super::hgrun::Run::start_run(run_id, pool)
        .await
        .with_context(|| format!("Impossible de démarrer le run {run_id}"))?;
    super::attempt::Attempt::start_run(run_id, pool)
        .await
        .with_context(|| format!("Impossible de démarrer la tentative du run {run_id}"))
}

/// Termine le run avec succès : Running -> Success (appelé par la routine de vérification)
pub async fn complete_success(run_id: i64, pool: &SqlitePool) -> Result<()> {
    let run: Run = Run::get_run_from_id(run_id, pool)
        .await
        .with_context(|| format!("Impossible de relire le run {run_id}"))?;
    if !matches!(run.status, RunStatus::Running) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Success.to_string(),
        }
        .into());
    }
    super::hgrun::Run::complete_success(run_id, pool)
        .await
        .with_context(|| format!("Impossible de terminer le run {run_id}"))?;
    super::attempt::Attempt::complete_success(run_id, pool)
        .await
        .with_context(|| format!("Impossible de terminer la tentative du run {run_id}"))
}

/// Termine le run avec échec : Running -> Failure (appelé par la routine de vérification)
pub async fn complete_failure(run_id: i64, reason: &str, pool: &SqlitePool) -> Result<()> {
    let reason = reason.to_string();
    let run: Run = Run::get_run_from_id(run_id, pool)
        .await
        .with_context(|| format!("Impossible de relire le run {run_id}"))?;
    if !matches!(run.status, RunStatus::Running) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Failure(reason).to_string(),
        }
        .into());
    }
    super::hgrun::Run::complete_failure(run_id, &reason, pool)
        .await
        .with_context(|| format!("Impossible de terminer le run {run_id} en échec"))?;
    super::attempt::Attempt::complete_failure(run_id, &reason, pool)
        .await
        .with_context(|| format!("Impossible de terminer la tentative du run {run_id} en échec"))
}

/// Transitionne un run en échec si le run est introuvable, ou toute ature erreur empêchant le lancement : Pending -> Failure (appelé par la routine de vérification)
pub async fn fail_cannot_analyse_run(run_id: i64, reason: &str, pool: &SqlitePool) -> Result<()> {
    let run = Run::get_run_from_id(run_id, pool)
        .await
        .with_context(|| format!("Impossible de relire le run {run_id}"))?;
    if !matches!(run.status, RunStatus::Pending) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Failure(reason.to_string()).to_string(),
        }
        .into());
    }
    super::hgrun::Run::complete_failure(run_id, reason, pool)
        .await
        .with_context(|| format!("Impossible de marquer le run {run_id} en échec"))?;
    super::attempt::Attempt::complete_failure(run_id, reason, pool)
        .await
        .with_context(|| format!("Impossible de marquer la tentative du run {run_id} en échec"))?;
    Ok(())
}

/// Commenter une tentative
pub async fn comment_attempt(
    run_id: i64,
    attempt_number: u32,
    comment: &str,
    pool: &SqlitePool,
) -> Result<()> {
    let attempt: Attempt = Attempt::get_attempt_from_number(attempt_number, run_id, pool)
        .await
        .with_context(|| {
            format!("Impossible de lire la tentative {attempt_number} du run {run_id}")
        })?;
    if !matches!(attempt.status, RunStatus::Success | RunStatus::Failure(_)) {
        return Err(AnalysisStateMachineError::InvalidOperation(
            "Vous ne pouvez commenter que des tentatives terminées".to_string(),
        )
        .into());
    }
    Attempt::update_comment(run_id, attempt_number, comment, pool)
        .await
        .with_context(|| {
            format!("Impossible de commenter la tentative {attempt_number} du run {run_id}")
        })?;
    Ok(())
}

/// Relance le run : Success/Failure -> Idle
pub async fn relaunch_run(run_id: i64, pool: &SqlitePool) -> Result<()> {
    let run: Run = Run::get_run_from_id(run_id, pool)
        .await
        .with_context(|| format!("Impossible de relire le run {run_id}"))?;
    if !matches!(run.status, RunStatus::Success | RunStatus::Failure(_)) {
        return Err(AnalysisStateMachineError::InvalidTransition {
            from: run.status.to_string(),
            to: RunStatus::Idle.to_string(),
        }
        .into());
    }
    super::hgrun::Run::relaunch_run(run_id, pool)
        .await
        .with_context(|| format!("Impossible de relancer le run {run_id}"))
}
