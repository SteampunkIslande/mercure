use crate::models::Form;
use crate::models::User;

use crate::models::ModelError;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::str::FromStr;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RunDefinitionError {
    #[error("Le run est dans un statut invalide")]
    InvalidRunStatusError,
    #[error("RunID manquant!")]
    MissingRunID,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum RunStatus {
    /// Le formulaire a été créé mais pas encore validé
    #[default]
    Idle,
    /// Le formulaire a été validé mais l'analyse n'a pas encore commencé
    Pending,
    /// Le formulaire a été validé et l'analyse est en cours
    Running,
    /// Le formulaire a été validé et l'analyse s'est terminée avec succès
    Success,
    /// Le formulaire a été validé mais l'analyse a échoué avec une erreur
    Failure(String),
}

impl Display for RunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunStatus::Idle => write!(f, "Idle"),
            RunStatus::Pending => write!(f, "Pending"),
            RunStatus::Running => write!(f, "Running"),
            RunStatus::Success => write!(f, "Success"),
            RunStatus::Failure(reason) => write!(f, "Failure:{}", reason),
        }
    }
}

impl FromStr for RunStatus {
    type Err = ModelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Idle" => Ok(RunStatus::Idle),
            "Pending" => Ok(RunStatus::Pending),
            "Running" => Ok(RunStatus::Running),
            "Success" => Ok(RunStatus::Success),
            s if s.starts_with("Failure:") => Ok(RunStatus::Failure(s[8..].to_string())),
            _ => Err(RunDefinitionError::InvalidRunStatusError.into()),
        }
    }
}

/// Created by users.
/// On any user's home page, there is a list of runs submitted by the user
/// There is also a button that the user can press to get to route '/newrun/groupname'
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Run {
    /// Auto-incremented Run ID (database defined).
    pub run_id: Option<i64>,
    /// Id of the user who created this run. Not modifiable with new attempts.
    pub user_id: i64,

    /// A user-defined name for this run
    /// Not modifiable with new attempts.
    pub run_name: String,

    /// Date of creation of this run in the database. Not modifiable with new attempts.
    pub creation_date: String,

    /// Current status of the run. Reflects the status of the latest attempt.
    pub status: RunStatus,

    /// Number of attempts made for this run. Incremented each time a new attempt is created.
    pub attempt_count: u32,

    /// User defined variables
    pub user_defined_vars: HashMap<String, String>,

    /// Path to the yaml file within said `branch`
    pub form_path: PathBuf,

    /// Name of the branch this run will take its code from
    pub branch_name: String,
}

impl Run {
    /// Crée un nouveau HgRun à partir de l'ID d'un HgFormDef et d'autres paramètres nécessaires
    pub async fn new_run(mut run_submission: Run, pool: &SqlitePool) -> Result<i64, ModelError> {
        // Date de création (peu importe ce que l'utilisateur avait soumis)
        run_submission.creation_date = OffsetDateTime::now_utc().to_string();

        // Insérer dans la table Runs
        let user_defined_vars = serde_json::to_string(&run_submission.user_defined_vars)?;

        let run_id = sqlx::query(
            r#"
            INSERT INTO Runs (user_id, run_name, creation_date, status, attempt_count, branch_name, user_defined_vars, form_path)
            VALUES (?,?,?,?,?,?,?,?)
            RETURNING run_id
            "#,
        )
        .bind(&run_submission.user_id)
        .bind(&run_submission.run_name)
        .bind(&run_submission.creation_date)
        .bind(RunStatus::Idle.to_string())
        .bind(0)
        .bind(&run_submission.branch_name)
        .bind(&user_defined_vars)
        .bind(run_submission.form_path.display().to_string())
        .fetch_one(pool)
        .await?
        .try_get("run_id")?;

        Ok(run_id)
    }

    /// Edite un HgRun à partir de son ID et d'autres paramètres nécessaires
    pub async fn edit_run(
        run_id: i64,
        user_defined_vars: HashMap<String, String>,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        let user_defined_vars = serde_json::to_string(&user_defined_vars)?;

        let run: Run = Self::get_run_from_id(run_id, pool).await?;
        if !matches!(run.status, RunStatus::Idle) {
            return Err(ModelError::FormError(
                "Impossible d'éditer un run validé, en cours d'analyse, ou terminé".to_string(),
            ));
        }

        sqlx::query(
            r#"
            UPDATE Runs SET user_defined_vars = ? WHERE run_id = ?
            "#,
        )
        .bind(&user_defined_vars)
        .bind(run_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Instancie un HgRun à partir de son run_id en le récupérant depuis la base de données
    pub async fn get_run_from_id(run_id: i64, pool: &SqlitePool) -> Result<Self, ModelError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM Runs WHERE run_id = ?
            "#,
        )
        .bind(run_id)
        .fetch_one(pool)
        .await?;

        let run_name: String = row.try_get("run_name")?;
        let creation_date: String = row.try_get("creation_date")?;

        let user_id: i64 = row.try_get("user_id")?;
        // Récupérer l'utilisateur, juste pour s'assurer que le user_id est valide
        let _ = sqlx::query_as::<_, User>(
            r#"
            SELECT id, usermail, username, password_hash, created_at, last_login, is_admin
            FROM Users WHERE id = ?
            "#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(ModelError::FormError("Utilisateur non trouvé".to_string()))?;

        // Désérialiser les variables définies par l'utilisateur
        let user_defined_vars: HashMap<String, String> =
            serde_json::from_str(row.try_get("user_defined_vars")?)?;

        // Parser le statut
        let status: RunStatus = RunStatus::from_str(row.try_get("status")?)?;
        // Nombre de tentatives
        let attempt_count = row.try_get("attempt_count")?;
        // Le nom de la branche à utiliser pour l'exécution
        let branch_name: String = row.try_get("branch_name")?;

        let form_path = PathBuf::from_str(row.try_get("form_path")?)?;

        // Construire l'instance HgRun
        let hgrun = Run {
            run_id: Some(run_id),
            user_id,
            run_name,
            creation_date,
            status,
            attempt_count,
            branch_name,
            user_defined_vars,
            form_path,
        };

        Ok(hgrun)
    }

    pub async fn get_form(&self) -> Result<Form, ModelError> {
        Form::get_form(&self.branch_name, &self.form_path).await
    }

    pub async fn get_user(&self, pool: &SqlitePool) -> Result<User, ModelError> {
        Ok(User::find_by_id(self.user_id, pool).await?)
    }

    /// Valide le formulaire pour ce run : Idle -> Pending
    ///
    /// Cette fonction ne modifie que la table HgRun.
    ///
    /// Il est de la responsabilité de l'appelant de mettre à jour la table Attempts **après** cette fonction.
    pub(super) async fn validate_form(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        // Incrémenter le nombre de tentatives
        sqlx::query(
            r#"
            UPDATE Runs SET (attempt_count, status) = (attempt_count + 1, ?) WHERE run_id = ?
            "#,
        )
        .bind(RunStatus::Pending.to_string())
        .bind(run_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Démarre le run : Pending -> Running (appelé par la routine de surveillance)
    pub(super) async fn start_run(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        // Mettre à jour le statut à Running
        sqlx::query(
            r#"
            UPDATE Runs SET status = ? WHERE run_id = ?
            "#,
        )
        .bind(RunStatus::Running.to_string())
        .bind(run_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Termine le run avec succès : Running -> Success
    pub(super) async fn complete_success(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        // Mettre à jour le statut à Success
        sqlx::query(
            r#"
            UPDATE Runs SET status = ? WHERE run_id = ?
            "#,
        )
        .bind(RunStatus::Success.to_string())
        .bind(run_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Termine le run avec échec : Running -> Failure
    pub(super) async fn complete_failure(
        run_id: i64,
        reason: &str,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        // Mettre à jour le statut à Failure
        sqlx::query(
            r#"
            UPDATE Runs SET status = ? WHERE run_id = ?
            "#,
        )
        .bind(RunStatus::Failure(reason.to_string()).to_string())
        .bind(run_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Relance le run : Success/Failure -> Idle
    pub(super) async fn relaunch_run(run_id: i64, pool: &SqlitePool) -> Result<(), ModelError> {
        // Mettre à jour le statut à Idle
        sqlx::query(
            r#"
            UPDATE Runs SET status = ? WHERE run_id = ?
            "#,
        )
        .bind("Idle")
        .bind(run_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
