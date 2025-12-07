use crate::models::User;

use super::form::HgFormDef;
use crate::models::ModelError;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub struct InvalidRunStatusError;

impl Display for InvalidRunStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid run status")
    }
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
    type Err = InvalidRunStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Idle" => Ok(RunStatus::Idle),
            "Pending" => Ok(RunStatus::Pending),
            "Running" => Ok(RunStatus::Running),
            "Success" => Ok(RunStatus::Success),
            s if s.starts_with("Failure:") => Ok(RunStatus::Failure(s[8..].to_string())),
            _ => Err(InvalidRunStatusError),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HgRunSubmission {
    pub form_id: i64,
    pub user_id: i64,
    pub run_name: String,
    pub run_date: String,
    pub run_sequencer: String,
    pub run_flowcellid: String,
    pub sample_sheet_adn_path: String,
    pub sample_sheet_arn_path: String,
    pub metadata_path: String,
    pub user_defined_vars: HashMap<String, String>,
    pub indir: Option<String>,
    pub outdir: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HgRunEdit {
    pub run_id: i64,

    pub run_date: String,
    pub run_sequencer: String,
    pub run_flowcellid: String,
    pub sample_sheet_adn_path: String,
    pub sample_sheet_arn_path: String,
    pub metadata_path: String,
    pub user_defined_vars: HashMap<String, String>,
    pub indir: Option<String>,
    pub outdir: Option<String>,
}

/// Created by users.
/// On any user's home page, there is a list of runs submitted by the user
/// There is also a button that the user can press to get to route '/newrun/groupname'
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HgRun {
    pub run_id: i64,

    /// The form definition used to create this run
    /// Not modifiable with new attempts, defines the pipeline used.
    pub form: HgFormDef,

    /// The user who created this run. Not modifiable with new attempts.
    pub user: User,

    /// The key,value pairs for user-defined variables. Editable from attempt to attempt
    /// Comes from the form definition, once the user has defined values for them.
    pub user_defined_vars: HashMap<String, String>,

    /// A user-defined name for this run
    /// Not modifiable with new attempts.
    pub run_name: String,

    /// Date of the run, as defined by the user. Can be modified with new attempts.
    pub run_date: String,
    /// The sequencer used for this run, as defined by the user. Can be modified with new attempts.
    pub run_sequencer: String,
    /// The flowcell ID used for this run, as defined by the user. Can be modified with new attempts.
    pub run_flowcellid: String,

    /// Date of creation of this run in the database. Not modifiable with new attempts.
    pub creation_date: String,

    /// Paths to the SampleSheets and metadata file, as defined by the user. Can be modified with new attempts.
    pub sample_sheet_adn_path: String,
    pub sample_sheet_arn_path: String,
    pub metadata_path: String,

    /// Input and output directories for the run, determined based on form's indir_type
    pub indir: Option<String>,
    pub outdir: Option<String>,

    /// Current status of the run. Reflects the status of the latest attempt.
    pub status: RunStatus,

    /// Number of attempts made for this run. Incremented each time a new attempt is created.
    pub attempt_count: u32,
}

impl HgRun {
    /// Crée un nouveau HgRun à partir de l'ID d'un HgFormDef et d'autres paramètres nécessaires
    pub async fn new_run(run_form: HgRunSubmission, pool: &SqlitePool) -> Result<i64, ModelError> {
        // Date de création
        let creation_date = OffsetDateTime::now_utc().to_string();

        // Insérer dans la table Runs
        let user_defined_vars_json = serde_json::to_string(&run_form.user_defined_vars)
            .map_err(|e| ModelError::FormError(format!("Erreur de sérialisation JSON: {}", e)))?;

        if run_form.sample_sheet_adn_path.is_empty() && run_form.sample_sheet_arn_path.is_empty() {
            return Err(ModelError::FormError(
                "Erreur de soumission d'un run: au moins une SampleSheet est requise".to_string(),
            ));
        }

        let run_id=sqlx::query(
            r#"
            INSERT INTO Runs (form_id, user_id, run_name, run_date, creation_date, run_sequencer, run_flowcellid, sample_sheet_adn_path, sample_sheet_arn_path, metadata_path, status, user_defined_vars, attempt_count, indir, outdir)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING run_id
            "#,
        )
        .bind(run_form.form_id)
        .bind(run_form.user_id)
        .bind(&run_form.run_name)
        .bind(&run_form.run_date)
        .bind(&creation_date)
        .bind(&run_form.run_sequencer)
        .bind(&run_form.run_flowcellid)
        .bind(&run_form.sample_sheet_adn_path)
        .bind(&run_form.sample_sheet_arn_path)
        .bind(&run_form.metadata_path)
        .bind("Idle")
        .bind(&user_defined_vars_json)
        .bind(0)
        .bind(&run_form.indir)
        .bind(&run_form.outdir)
        .fetch_one(pool)
        .await?.try_get("run_id")?;

        Ok(run_id)
    }

    /// Edite un HgRun à partir de son ID et d'autres paramètres nécessaires
    pub async fn edit_run(run_form: HgRunEdit, pool: &SqlitePool) -> Result<i64, ModelError> {
        eprintln!("Received run edit {:?}", run_form);
        let user_defined_vars_json = serde_json::to_string(&run_form.user_defined_vars)
            .map_err(|e| ModelError::FormError(format!("Erreur de sérialisation JSON: {}", e)))?;

        if run_form.sample_sheet_adn_path.is_empty() && run_form.sample_sheet_arn_path.is_empty() {
            return Err(ModelError::FormError(
                "Erreur de soumission d'un run: au moins une SampleSheet est requise".to_string(),
            ));
        }

        let run: HgRun = Self::get_run_from_id(run_form.run_id, pool).await?;
        if !matches!(run.status, RunStatus::Idle) {
            return Err(ModelError::FormError(
                "Impossible d'éditer un run validé, en cours d'analyse, ou terminé".to_string(),
            ));
        }

        sqlx::query(
            r#"
            UPDATE Runs SET
            (user_defined_vars, run_date, run_sequencer, run_flowcellid, sample_sheet_adn_path, sample_sheet_arn_path, metadata_path, indir, outdir)
            =
            (?, ?, ?, ?, ?, ?, ?, ?, ?)
            WHERE run_id = ?
            "#,
        )
        .bind(&user_defined_vars_json)
        .bind(&run_form.run_date)
        .bind(&run_form.run_sequencer)
        .bind(&run_form.run_flowcellid)
        .bind(&run_form.sample_sheet_adn_path)
        .bind(&run_form.sample_sheet_arn_path)
        .bind(&run_form.metadata_path)
        .bind(&run_form.indir)
        .bind(&run_form.outdir)
        .bind(run_form.run_id)
        .execute(pool).await?;

        Ok(run_form.run_id)
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

        let form_id: i64 = row.try_get("form_id")?;
        let user_id: i64 = row.try_get("user_id")?;

        // Récupérer la définition du formulaire
        let form = HgFormDef::get_formdef_from_id(pool, form_id).await?;

        // Récupérer l'utilisateur
        let user = sqlx::query_as::<_, User>(
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
        let user_defined_vars_json: String = row.try_get("user_defined_vars")?;
        let user_defined_vars: HashMap<String, String> =
            serde_json::from_str(&user_defined_vars_json).map_err(|e| {
                ModelError::FormError(format!("Erreur de désérialisation JSON: {}", e))
            })?;

        // Parser le statut
        let status: RunStatus = RunStatus::from_str(row.try_get("status")?)?;

        // Construire l'instance HgRun
        let hgrun = HgRun {
            run_id,
            form,
            user,
            user_defined_vars,
            run_name: row.try_get("run_name")?,
            run_date: row.try_get("run_date")?,
            creation_date: row.try_get("creation_date")?,
            run_sequencer: row.try_get("run_sequencer")?,
            run_flowcellid: row.try_get("run_flowcellid")?,
            sample_sheet_adn_path: row.try_get("sample_sheet_adn_path")?,
            sample_sheet_arn_path: row.try_get("sample_sheet_arn_path")?,
            metadata_path: row.try_get("metadata_path")?,
            indir: row.try_get("indir").ok(),
            outdir: row.try_get("outdir").ok(),
            status,
            attempt_count: row.try_get("attempt_count")?,
        };

        Ok(hgrun)
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
        .bind("Pending")
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
        .bind("Running")
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
        .bind("Success")
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
        .bind(format!("Failure:{}", reason))
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
