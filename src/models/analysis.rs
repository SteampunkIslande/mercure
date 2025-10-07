use super::ModelError;
use sqlx::SqlitePool;

/// The state machine to control Run and Attempt API
/// The only way to interact with the state of the database regarding Runs and Attempts
pub struct AnalysisStateMachine {}

impl AnalysisStateMachine {
    // === HgRun methods ===

    /// Crée un nouveau HgRun à partir de l'ID d'un HgFormDef et d'autres paramètres nécessaires
    pub async fn new_run(
        run_form: super::hgrun::HgRunSubmission,
        pool: &SqlitePool,
    ) -> Result<i64, ModelError> {
        super::hgrun::HgRun::new_run(run_form, pool).await
    }

    /// Valide le formulaire pour ce run : Idle -> Pending, crée une nouvelle tentative
    pub async fn validate_form(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        super::hgrun::HgRun::validate_form(run_id, pool).await?;
        super::attempt::HgAttempt::new_attempt(run_id, pool).await
    }

    /// Démarre le run : Pending -> Running (appelé par la routine de vérification)
    pub async fn start_run_analysis(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        super::hgrun::HgRun::start_run(run_id, pool).await
    }

    /// Termine le run avec succès : Running -> Success
    pub async fn complete_success(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        super::hgrun::HgRun::complete_success(run_id, pool).await
    }

    /// Termine le run avec échec : Running -> Failure
    pub async fn complete_failure(
        run_id: i64,
        reason: String,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        super::hgrun::HgRun::complete_failure(run_id, reason, pool).await
    }

    /// Relance le run : Success/Failure -> Idle
    pub async fn relaunch_run(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        super::hgrun::HgRun::relaunch_run(run_id, pool).await
    }

    // === HgAttempt methods ===

    /// Crée une nouvelle tentative à partir d'une HgRun
    pub async fn new_attempt(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        super::attempt::HgAttempt::new_attempt(run_id, pool).await
    }

    /// Instancie un HgAttempt à partir de son attempt_number et run_id
    pub async fn get_attempt_from_number(
        attempt_number: i64,
        run_id: i64,
        pool: &SqlitePool,
    ) -> Result<super::attempt::HgAttempt, ModelError> {
        super::attempt::HgAttempt::get_attempt_from_number(attempt_number, run_id, pool).await
    }

    /// Récupère toutes les tentatives pour un run_id
    pub async fn get_attempts_for_run(
        run_id: i64,
        pool: &SqlitePool,
    ) -> Result<Vec<super::attempt::HgAttempt>, ModelError> {
        super::attempt::HgAttempt::get_attempts_for_run(run_id, pool).await
    }

    /// Termine toutes les tentatives avec échec : Running -> Failure
    pub async fn complete_attempts_failure_for_run(
        run_id: i64,
        reason: String,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        super::attempt::HgAttempt::set_attempt_failure_for_run(run_id, reason, pool).await
    }

    /// Met à jour le commentaire d'une tentative
    pub async fn update_comment(
        attempt_id: i64,
        new_comment: String,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        super::attempt::HgAttempt::update_comment(attempt_id, new_comment, pool).await
    }
}
