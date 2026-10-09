use crate::models::CachedForm;
use crate::models::Form;
use crate::models::Group;
use crate::models::User;

use crate::models::ModelError;
use crate::utils::format_french_date;
use anyhow::Context;
use rocket::form::FromFormField;
use rocket::form::ValueField;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::Value;
use serde_json::json;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;
use time::{OffsetDateTime, UtcOffset};

#[derive(Debug, thiserror::Error)]
pub enum RunDefinitionError {
    #[error("Le run est dans un statut invalide")]
    InvalidRunStatusError,
    #[error("Le run ne peut pas être édité, seul un run 'A valider' peut l'être")]
    UneditableRun,
    #[error("RunID manquant!")]
    MissingRunID,
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    AnyHowError(#[from] anyhow::Error),
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

#[rocket::async_trait]
impl<'r> FromFormField<'r> for RunStatus {
    fn default() -> Option<Self> {
        Some(RunStatus::Idle)
    }
    fn from_value(field: ValueField<'r>) -> rocket::form::Result<'r, Self> {
        // Failure seul (filtre du front) désigne tous les statuts d'échec :
        // la raison est vide, et le filtre préfixe `Failure:%` les couvre tous
        if field.value == "Failure" {
            return Ok(RunStatus::Failure(String::new()));
        }

        Ok(Self::from_str(field.value)
            .map_err(|_| rocket::form::Error::validation("Statut de run invalide"))?)
    }
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
    type Err = RunDefinitionError;

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
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub struct Run {
    /// Auto-incremented Run ID (database defined).
    pub run_id: Option<i64>,
    /// Id of the user who created this run. Not modifiable with new attempts.
    pub user_id: i64,

    /// A user-defined name for this run
    /// Not modifiable with new attempts.
    pub run_name: String,

    /// Date of creation of this run in the database. Not modifiable with new attempts.
    pub creation_date: Option<OffsetDateTime>,

    /// Current status of the run. Reflects the status of the latest attempt.
    pub status: RunStatus,

    /// Number of attempts made for this run. Incremented each time a new attempt is created.
    pub attempt_count: u32,

    /// User defined variables
    pub user_defined_vars: HashMap<String, String>,

    /// Path to the yaml file within said `branch`
    pub form_path: String,

    /// Name of the branch this run will take its code from
    pub branch_name: String,
}

impl Run {
    /// Crée un nouveau HgRun à partir de l'ID d'un HgFormDef et d'autres paramètres nécessaires
    pub async fn new_run(mut run_submission: Run, pool: &SqlitePool) -> Result<i64, ModelError> {
        // Date de création (peu importe ce que l'utilisateur avait soumis)
        run_submission.creation_date = Some(OffsetDateTime::now_utc());

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
        .bind(&run_submission.form_path)
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
    ) -> Result<(), RunDefinitionError> {
        let user_defined_vars = serde_json::to_string(&user_defined_vars)?;

        let run: Run = Self::get_run_from_id(run_id, pool).await?;
        if !matches!(run.status, RunStatus::Idle) {
            return Err(RunDefinitionError::UneditableRun);
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

    /// Sérialise le run en une ligne de tableau pour le front.
    ///
    /// La sortie doit correspondre colonne par colonne à `Run::runs_table_header`.
    fn to_json(&self, user_name: &str) -> Option<Value> {
        let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let creation_date = self
            .creation_date
            .unwrap_or(OffsetDateTime::now_local().unwrap_or(OffsetDateTime::now_utc()))
            .to_offset(local_offset);
        let creation_date_str = format_french_date(&creation_date, false);
        Some(json!([
            // Colonne 1 - Nom du run
            json!({
                "content": &self.run_name,
                "href": Some(format!("/mercure/show/run/{}", self.run_id?)),
                "td_class": "content-column"
            }),
            // Colonne 2 - Utilisateur
            json!({
                "content": user_name,
                "td_class": "content-column"
            }),
            // Colonne 3 - Date du run
            json!({
                "content": creation_date_str,
                "td_class": "content-column"
            }),
            // Colonne 4 - Statut
            json!({
                "content": match self.status {
                    RunStatus::Idle => "A valider",
                    RunStatus::Failure(_) => "Echec",
                    RunStatus::Pending => "En attente",
                    RunStatus::Running => "Analyses en cours",
                    RunStatus::Success => "Succès"
                },
                "class": format!("run-status {}", match self.status {
                    RunStatus::Idle => "idle",
                    RunStatus::Failure(_) => "failure",
                    RunStatus::Pending => "pending",
                    RunStatus::Running => "running",
                    RunStatus::Success => "success"
                }),
                "td_class": "badge-column"
            }),
            // Colonne 5 - Tentatives
            json!({
                "content": &self.attempt_count.to_string(),
                "td_class": "numeric-column"
            })
        ]))
    }

    /// Entête du tableau des runs.
    ///
    /// La sortie doit correspondre colonne par colonne à `Run::to_json`.
    fn runs_table_header() -> Vec<Value> {
        vec![
            json!({"content": "Nom du run", "class": "content-column"}),
            json!({"content": "Utilisateur", "class": "content-column"}),
            json!({"content": "Date du run", "class": "content-column"}),
            json!({"content": "Statut", "class": "badge-column"}),
            json!({"content": "Tentative", "class": "numeric-column"}),
        ]
    }

    /// Construit le JSON attendu par le front : titre, en-tête de tableau,
    /// lignes de runs paginées et pagination.
    fn runs_payload(
        title: &str,
        table: Vec<Value>,
        current_page: i64,
        page_size: i64,
        total_count: i64,
    ) -> Value {
        let total_pages = ((total_count + page_size - 1) / page_size).max(1);

        json!({
            "title": title,
            "header": Self::runs_table_header(),
            "table": table,
            "pagination": {
                "current_page": current_page,
                "total_pages": total_pages,
                "page_size": page_size,
                "total_count": total_count
            }
        })
    }

    /// Noyau SQL commun à `Run::list_runs` et `Run::search_runs`.
    ///
    /// Construit dynamiquement la clause WHERE à partir :
    ///
    /// 1. de la visibilité : seul un admin (`user = None`) voit tous les runs, un
    ///    utilisateur standard ne voit que les runs des formulaires associés à au moins
    ///    un de ses groupes (les paires `branch_name`/`form_path` étant résolues depuis
    ///    le cache `FormDef`/`FormDefHasGroup`, voir `Run::visible_form_pairs`) ;
    /// 2. des filtres optionnels passés dans `filters`.
    ///
    /// Renvoie le JSON prêt pour le front : `{title, header, table, pagination}`.
    async fn fetch_runs_table(
        pool: &SqlitePool,
        user: Option<&User>,
        groups: Option<Vec<Group>>,
        page_size: Option<i64>,
        page: Option<i64>,
        title: &str,
        filters: RunFilters<'_>,
    ) -> Result<Value, ModelError> {
        let limit = page_size.unwrap_or(DEFAULT_RUNS_PAGE_SIZE).max(1);
        let current_page = page.unwrap_or(1).max(1);
        let offset = (current_page - 1) * limit;

        let mut where_parts: Vec<String> = Vec::new();
        let mut where_binds: Vec<String> = Vec::new();

        // 1. Visibilité par groupes
        if let Some(u) = user {
            let viewer_groups = match groups {
                Some(groups) => groups,
                // Repli : relire les groupes de l'utilisateur depuis la base
                None => Group::get_user_groups(pool, u.id).await?,
            };

            let form_pairs = CachedForm::visible_form_pairs(pool, viewer_groups).await?;

            if form_pairs.is_empty() {
                // L'utilisateur n'a accès à aucun formulaire : liste vide
                return Ok(Self::runs_payload(
                    title,
                    Vec::new(),
                    current_page,
                    limit,
                    0,
                ));
            }

            where_parts.push(format!(
                "({})",
                form_pairs
                    .iter()
                    .map(|_| "(r.branch_name = ? AND r.form_path = ?)".to_string())
                    .collect::<Vec<_>>()
                    .join(" OR ")
            ));
            for (branch, form_path) in &form_pairs {
                where_binds.push(branch.clone());
                where_binds.push(form_path.clone());
            }
        }

        // 2. Filtres optionnels
        if let Some(status) = filters.status {
            where_parts.push("r.status LIKE ?".to_string());
            where_binds.push(format!("{status}%"));
        }
        if let Some(date_from) = filters.date_from {
            where_parts.push("r.creation_date >= ?".to_string());
            where_binds.push(date_from.to_string());
        }
        if let Some(date_to) = filters.date_to {
            where_parts.push("r.creation_date <= ?".to_string());
            where_binds.push(date_to.to_string());
        }
        if let Some(run_name_search) = filters.run_name_search {
            where_parts.push("r.run_name LIKE ?".to_string());
            where_binds.push(format!("%{run_name_search}%"));
        }

        let where_sql = if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        };

        // Compte total pour la pagination
        let count_query_str = format!("SELECT COUNT(r.run_id) as total FROM Runs r{where_sql}");

        // Résultats paginés, triés par date de création, les plus récents d'abord
        let base_query_str = format!(
            "SELECT r.run_id, r.user_id, r.run_name, r.creation_date, r.status,
             r.attempt_count, r.user_defined_vars, r.form_path, r.branch_name, u.username
             FROM Runs r
             INNER JOIN Users u ON r.user_id = u.id{where_sql}
             ORDER BY r.creation_date DESC
             LIMIT ? OFFSET ?"
        );

        let mut count_query = sqlx::query(&count_query_str);
        let mut base_query = sqlx::query(&base_query_str);
        for bind in &where_binds {
            count_query = count_query.bind(bind);
            base_query = base_query.bind(bind);
        }

        let total_count: i64 = count_query.fetch_one(pool).await?.try_get("total")?;

        let rows = base_query.bind(limit).bind(offset).fetch_all(pool).await?;

        let mut table = Vec::with_capacity(rows.len());
        for row in &rows {
            if let Some(row_json) = run_row_to_json(row) {
                table.push(row_json);
            }
        }

        Ok(Self::runs_payload(
            title,
            table,
            current_page,
            limit,
            total_count,
        ))
    }

    /// Liste les runs visibles, paginés et triés par date de création décroissante.
    ///
    /// Un utilisateur non-admin ne voit que les runs des formulaires associés à au
    /// moins un de ses groupes ; passer `user = None` (admin) désactive ce filtre.
    ///
    /// # Arguments
    ///
    /// - `pool` - connexion à la base de données
    /// - `user` - l'utilisateur demandeur ; `None` signifie `admin`, donc voit tout
    /// - `groups` - les groupes dont l'utilisateur est membre ; si `None` pour un
    ///   utilisateur non-admin, ses groupes sont relus depuis la base
    /// - `page_size` - nombre de runs par page (défaut : 5)
    /// - `page` - page à afficher, à partir de 1 (défaut : 1)
    /// - `status` - filtre optionnel de statut, appliqué en préfixe
    ///   (`LIKE '{status}%'`, ce qui couvre aussi `Failure:raison`) ; `None` n'applique aucun filtre
    ///
    /// # Returns
    ///
    /// - Le JSON prêt à consommer par le front : `{title, header, table, pagination}`
    ///
    /// # Errors
    ///
    /// En cas d'erreur SQL, d'erreur de lecture du cache des formulaires (`FormDef`), ou
    /// d'erreur de lecture des groupes de l'utilisateur si `groups` vaut `None`.
    pub async fn list_runs(
        pool: &SqlitePool,
        user: Option<&User>,
        page_size: Option<i64>,
        page: Option<i64>,
        status: Option<RunStatus>,
        groups: Option<Vec<Group>>,
    ) -> Result<Value, ModelError> {
        let title = if user.is_none() {
            "Liste des runs (tous les groupes)"
        } else {
            "Liste des runs de vos groupes"
        };

        let status_str = status.map(|s| s.to_string());

        Self::fetch_runs_table(
            pool,
            user,
            groups,
            page_size,
            page,
            title,
            RunFilters {
                status: status_str.as_deref(),
                date_from: None,
                date_to: None,
                run_name_search: None,
            },
        )
        .await
    }

    /// Recherche paginée de runs visibles, triés par date de création décroissante.
    ///
    /// Même visibilité que `Run::list_runs` : un utilisateur non-admin ne voit que les
    /// runs des formulaires associés à au moins un de ses groupes.
    ///
    /// # Arguments
    ///
    /// - `pool` - connexion à la base de données
    /// - `user` - l'utilisateur demandeur ; `None` signifie « admin, voit tout »
    /// - `groups` - les groupes dont l'utilisateur est membre ; si `None` pour un
    ///   utilisateur non-admin, ses groupes sont relus depuis la base
    /// - `page_size` - nombre de runs par page (défaut : 5)
    /// - `page` - page à afficher, à partir de 1 (défaut : 1)
    /// - `status` - préfixe de statut recherché (`LIKE '{status}%'`)
    /// - `date_from` / `date_to` - bornes de dates de création au format `YYYY-MM-DD`
    /// - `run_name_search` - sous-chaîne à retrouver dans le nom du run
    ///
    /// # Returns
    ///
    /// - Le JSON prêt à consommer par le front : `{title, header, table, pagination}`
    ///
    /// # Errors
    ///
    /// En cas d'erreur SQL, d'erreur de lecture du cache des formulaires (`FormDef`), ou
    /// d'erreur de lecture des groupes de l'utilisateur si `groups` vaut `None`.
    #[allow(clippy::too_many_arguments)]
    pub async fn search_runs(
        pool: &SqlitePool,
        user: Option<&User>,
        page_size: Option<i64>,
        page: Option<i64>,
        status: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        run_name_search: Option<&str>,
        groups: Option<Vec<Group>>,
    ) -> Result<Value, ModelError> {
        let title = if user.is_none() {
            "Résultats de la recherche (tous les groupes)"
        } else {
            "Résultats de la recherche de vos groupes"
        };

        Self::fetch_runs_table(
            pool,
            user,
            groups,
            page_size,
            page,
            title,
            RunFilters {
                status,
                date_from,
                date_to,
                run_name_search,
            },
        )
        .await
    }

    /// Instancie un HgRun à partir de son run_id en le récupérant depuis la base de données
    pub async fn get_run_from_id(
        run_id: i64,
        pool: &SqlitePool,
    ) -> Result<Self, RunDefinitionError> {
        let row = sqlx::query(
            r#"
            SELECT * FROM Runs WHERE run_id = ?
            "#,
        )
        .bind(run_id)
        .fetch_one(pool)
        .await?;

        let run_name = row.try_get("run_name")?;
        let creation_date = row.try_get("creation_date")?;

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
        .context("Utilisateur non trouvé!")?;

        // Désérialiser les variables définies par l'utilisateur
        let user_defined_vars: HashMap<String, String> =
            serde_json::from_str(row.try_get("user_defined_vars")?)?;

        // Parser le statut
        let status: RunStatus = RunStatus::from_str(row.try_get("status")?)?;
        // Nombre de tentatives
        let attempt_count = row.try_get("attempt_count")?;
        // Le nom de la branche à utiliser pour l'exécution
        let branch_name: String = row.try_get("branch_name")?;

        let form_path: String = row.try_get("form_path")?;

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

/// Nombre de runs par page si `page_size` n'est pas fourni.
const DEFAULT_RUNS_PAGE_SIZE: i64 = 5;

/// Critères de filtrage partagés par `Run::list_runs` et `Run::search_runs`.
struct RunFilters<'a> {
    status: Option<&'a str>,
    date_from: Option<&'a str>,
    date_to: Option<&'a str>,
    run_name_search: Option<&'a str>,
}

/// Reconstruit un `Run` depuis une ligne de la requête de listing,
/// puis le sérialise pour le tableau du front.
fn run_row_to_json(row: &sqlx::sqlite::SqliteRow) -> Option<Value> {
    let run = Run {
        run_id: row.try_get("run_id").ok()?,
        user_id: row.try_get("user_id").ok()?,
        run_name: row.try_get("run_name").ok()?,
        creation_date: row.try_get("creation_date").ok()?,
        status: RunStatus::from_str(row.try_get("status").ok()?).ok()?,
        attempt_count: row.try_get("attempt_count").ok()?,
        user_defined_vars: serde_json::from_str(row.try_get("user_defined_vars").ok()?).ok()?,
        form_path: row.try_get("form_path").ok()?,
        branch_name: row.try_get("branch_name").ok()?,
    };
    let username: String = row.try_get("username").ok()?;

    run.to_json(&username)
}
