use crate::models::User;

use super::form::HgFormDef;
use crate::models::ModelError;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use time::OffsetDateTime;

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub enum RunStatus {
    /// Le formulaire a été créé mais pas encore validé
    #[default]
    Idle,
    /// Le formulaire a été validé mais l'analyse n'a pas encore commencé
    Pending,
    /// Le formulaire a été validé et l'analyse est en cours
    Running,
    /// Le formulaire a été validé mais l'analyse a échoué avec une erreur
    Failed(String),
    /// Le formulaire a été validé et l'analyse s'est terminée avec succès
    Success,
}

//TODO: Add a HgRunSubmission struct for the backend

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
}

/// Created by users.
/// On any user's home page, there is a list of runs submitted by the user
/// There is also a button that the user can press to get to route '/newrun/groupname'
///
/// This form is what is submitted by the user when they are on the '/newrun/groupname' GET endpoint
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HgRun {
    /// The form definition used to create this run
    pub form: HgFormDef,

    pub user: User,

    /// The key,value pairs for user-defined variables
    pub user_defined_vars: HashMap<String, String>,

    pub run_name: String,

    pub run_date: String,
    pub creation_date: String,
    pub run_sequencer: String,
    pub run_flowcellid: String,

    pub sample_sheet_adn_path: String,
    pub sample_sheet_arn_path: String,
    pub metadata_path: String,

    pub status: RunStatus,

    /// Md5Hash of the full zipped pipeline folder stored in the database as a BLOB
    /// Only determined at the time of launching the pipeline
    pub archived_folder_md5: Option<String>,
}

impl HgRun {
    /// Crée un nouveau HgRun à partir de l'ID d'un HgFormDef et d'autres paramètres nécessaires
    pub async fn new_run(run_form: HgRunSubmission, pool: &SqlitePool) -> Result<(), ModelError> {
        // Date de création
        let creation_date = OffsetDateTime::now_utc().to_string();

        // Insérer dans la table Runs
        let user_defined_vars_json = serde_json::to_string(&run_form.user_defined_vars)
            .map_err(|e| ModelError::FormError(format!("Erreur de sérialisation JSON: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO Runs (form_id, user_id, run_name, run_date, creation_date, run_sequencer, run_flowcellid, sample_sheet_adn_path, sample_sheet_arn_path, metadata_path, status, user_defined_vars, archived_folder_md5)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
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
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn remove_run(form_id: i64) -> Result<(), ModelError> {
        // TODO: Implement
        Ok(())
    }

    /// Instancie un HgRun à partir de son run_id en récupérant depuis la base de données
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
        let status_str: String = row.try_get("status")?;
        let status = match status_str.as_str() {
            "Idle" => RunStatus::Idle,
            "Pending" => RunStatus::Pending,
            "Running" => RunStatus::Running,
            "Success" => RunStatus::Success,
            s if s.starts_with("Failed:") => RunStatus::Failed(s[7..].to_string()),
            _ => RunStatus::Idle, // Défaut en cas d'inconnu
        };

        // Construire l'instance HgRun
        let hgrun = HgRun {
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
            status,
            archived_folder_md5: row.try_get("archived_folder_md5").ok(),
        };

        Ok(hgrun)
    }
}
