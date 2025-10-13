use chrono::Local;
use env_logger::Builder;
use log::{error, info};
use mercure::config::get_mercure_config;
use mercure::models::HgRun;
use mercure::models::analysis;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

// Init logger dès le démarrage, format date/heure local, niveau INFO, sortie stderr
fn init_logger() {
    Builder::new()
        .format(|buf, record| {
            let now = Local::now().format("%Y-%m-%d %H:%M:%S");
            writeln!(buf, "[{} {}] {}", record.level(), now, record.args())
        })
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Stderr)
        .init();
}

use mercure::models::AnalysisStateMachineError;
use sqlx;
use sqlx::Row;
use thiserror::Error;
use tokio::sync::watch;

use mercure::models::HgAttempt;
use mercure::models::InvalidRunStatusError;
use mercure::models::RunStatus;

#[derive(Error, Debug)]
enum RoutineError {
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error(transparent)]
    InvalidRunStatusError(#[from] InvalidRunStatusError),
    #[error(transparent)]
    AnalysisStateMachineError(#[from] AnalysisStateMachineError),
    #[error(transparent)]
    PathError(#[from] core::convert::Infallible),
    #[error(transparent)]
    GlobError(#[from] glob::PatternError),
    #[error(transparent)]
    DateParseError(#[from] chrono::ParseError),
    #[error("{0}")]
    CustomParseError(String),
    #[error(transparent)]
    ModelError(#[from] mercure::models::ModelError),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

// Ajout de la méthode utilitaire pour HgAttempt

/// Fonction qui exécute la boucle de routine avec des points de contrôle pour l'annulation
async fn run_routine_loop(
    pool: sqlx::SqlitePool,
    mut shutdown_rx: watch::Receiver<bool>,
) -> Result<(), RoutineError> {
    use std::time::Duration;

    loop {
        // Point de contrôle 1: Vérifier le signal d'arrêt au début de chaque itération
        if *shutdown_rx.borrow() {
            info!("Signal d'arrêt reçu, arrêt de la routine...");
            break;
        }

        // Recherche et traitement des runs en attente
        let pending_attempts = find_pending_runs(&pool).await;
        for attempt in pending_attempts {
            match treat_pending(&attempt, &pool).await {
                Ok(_) => {}
                Err(e) => {
                    error!(
                        "Erreur lors du traitement de la tentive {} du run {}: {e}",
                        attempt.attempt_number, attempt.run_id
                    );
                    analysis::fail_cannot_analyse_run(attempt.run_id, &e.to_string(), &pool)
                        .await?;
                }
            }
        }

        // Recherche et traitement des runs en cours
        let running_attempts = find_running_runs(&pool).await;
        for attempt in running_attempts {
            match treat_running(attempt, &pool).await {
                Ok(_) => {}
                Err(e) => error!("Erreur lors du traitement d'un run en cours d'analyse: {e}"),
            }
        }

        // Point de contrôle 2: Attendre avec possibilité d'interruption
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(300)) => {},
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() { break; }
            }
        }
    }
    Ok(())
}

// Cette fonction crée un fichier tel que spécifié dans le formulaire
// C'est HgRun qui a un membre dédié
async fn start_analysis(
    attempt: &HgAttempt,
    run_dir: &PathBuf,
    pool: &sqlx::SqlitePool,
) -> Result<(), RoutineError> {
    use std::fs;

    let config = get_mercure_config();
    let analysis_base_dir = PathBuf::from(&config.analysis_folder);

    // Extraction du nom brut du dossier de run
    let run_rawdir_basename =
        run_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(RoutineError::CustomParseError(
                "Failed to extract run basename".into(),
            ))?;

    // Extraction des composants
    let parts: Vec<&str> = run_rawdir_basename.splitn(4, '_').collect();
    if parts.len() < 4 {
        return Err(RoutineError::CustomParseError(format!(
            "Le nom du dossier de run '{}' ne contient pas assez de parties",
            run_rawdir_basename
        )));
    }
    let date = parts[0];
    let sequencer = parts[1];
    let number = parts[2];

    // Construction du nom du dossier d'analyse
    let analysis_dir_name = format!("{}_{}_{}", date, sequencer, number);
    let analysis_dir = analysis_base_dir.join(analysis_dir_name);

    // Création du dossier d'analyse si nécessaire
    if !analysis_dir.exists() {
        if let Err(e) = fs::create_dir_all(&analysis_dir) {
            error!("Erreur lors de la création du dossier d'analyse: {}", e);
            return Ok(());
        }
        info!("Dossier d'analyse créé: {}", analysis_dir.display());
    }

    // Obtention des informations sur le pipeline et le launcher choisis
    let run: HgRun = HgRun::get_run_from_id(attempt.run_id, pool).await?;

    // Le chemin du launcher doit être absolu. C'est ce chemin qui va se trouver dans le script généré
    let launcher_abs_path = format!(
        "{base}/{pipeline}/launchers/{launcher}",
        base = config.pipeline_dir,
        pipeline = run.form.pipeline_name,
        launcher = run.form.launcher_name
    );
    if !std::fs::exists(&launcher_abs_path)? {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Impossible de trouver le launcher au chemin suivant: {}",
                launcher_abs_path
            ),
        ))
        .map_err(RoutineError::from);
    }

    let script_path = format!(
        "{todo_dir}/jobs-{run_id}-{attempt_number}",
        todo_dir = config.todo_dir,
        run_id = attempt.run_id,
        attempt_number = attempt.attempt_number
    );
    if std::fs::exists(&script_path)? {
        return Err(RoutineError::IOError(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Le fichier {} existe déjà!", &script_path),
        )));
    }

    let exported_vars = attempt
        .user_defined_vars
        .iter()
        .map(|(k, v)| format!("export {k}={v}"))
        .chain([
            format!("export HG_RAWDIR={}", run_dir.display()),
            format!("export HG_ANALYSIS_DIR={}", analysis_dir.display()),
        ])
        .collect::<Vec<_>>()
        .join("\n");

    let mut script_file = std::fs::OpenOptions::new()
        .mode(0o775)
        .create(true)
        .open(&script_path)?;

    write!(
        &mut script_file,
        r#"#!/bin/bash

export PATH=/usr/bin:$PATH
{}
{}
"#,
        exported_vars, launcher_abs_path
    )?;

    Ok(())
}

/// Fonction pour trouver les runs en attente
/// Quelques effets de bord :
/// - Log les erreurs SQL
/// - Log les erreurs de récupération des tentatives
/// Retourne une liste vide en cas d'erreur
async fn find_pending_runs(pool: &sqlx::SqlitePool) -> Vec<HgAttempt> {
    match sqlx::query(
        "SELECT attempt_number, run_id FROM Attempts WHERE status = ? ORDER BY attempt_date ASC",
    )
    .bind(RunStatus::Pending.to_string())
    .fetch_all(pool)
    .await
    {
        Ok(rows) => {
            let mut attempts = Vec::new();
            for row in rows {
                let attempt_number = match row.try_get("attempt_number") {
                    Ok(num) => num,
                    Err(e) => {
                        error!("Erreur lors de la récupération du numéro de tentative: {e}");
                        continue;
                    }
                };
                let run_id = match row.try_get("run_id") {
                    Ok(id) => id,
                    Err(e) => {
                        error!("Erreur lors de la récupération de l'ID de run: {e}");
                        continue;
                    }
                };
                match HgAttempt::get_attempt_from_number(attempt_number, run_id, pool).await {
                    Ok(attempt) => attempts.push(attempt),
                    Err(e) => error!("Erreur lors de la récupération d'une tentative: {e}"),
                }
            }
            attempts
        }
        Err(e) => {
            error!("Erreur SQL lors de la récupération des runs en attente: {e}");
            vec![]
        }
    }
}

/// Fonction pour trouver les runs en cours d'analyse
/// Quelques effets de bord :
/// - Log les erreurs SQL
/// - Log les erreurs de récupération des tentatives
/// Retourne une liste vide en cas d'erreur
async fn find_running_runs(pool: &sqlx::SqlitePool) -> Vec<HgAttempt> {
    match sqlx::query(
        "SELECT attempt_number, run_id FROM Attempts WHERE status = ? ORDER BY attempt_date ASC",
    )
    .bind(RunStatus::Running.to_string())
    .fetch_all(pool)
    .await
    {
        Ok(rows) => {
            let mut attempts = Vec::new();
            for row in rows {
                let attempt_number = row.try_get("attempt_number").unwrap_or_default();
                let run_id = row.try_get("run_id").unwrap_or_default();
                match HgAttempt::get_attempt_from_number(attempt_number, run_id, pool).await {
                    Ok(attempt) => attempts.push(attempt),
                    Err(e) => {
                        error!("Erreur lors de la récupération d'une tentative running: {e}")
                    }
                }
            }
            attempts
        }
        Err(e) => {
            error!("Erreur SQL lors de la récupération des runs en cours: {e}");
            vec![]
        }
    }
}

fn get_supposed_run_dir_glob(run: &HgAttempt) -> String {
    let config = get_mercure_config();
    format!(
        "{raw}/{seq}/output/{date}_{seq}_*_{flowcellid}",
        raw = config.sequencers_folder,
        seq = run.run_sequencer,
        date = run.run_date[2..].replace("-", ""),
        flowcellid = run.run_flowcellid
    )
}

async fn treat_pending(attempt: &HgAttempt, pool: &sqlx::SqlitePool) -> Result<(), RoutineError> {
    use chrono::{Duration, Local, NaiveDate};
    let supposed_run_dir = get_supposed_run_dir_glob(&attempt);
    let paths: Vec<PathBuf> = glob::glob(&supposed_run_dir)
        .map_err(RoutineError::from)?
        .filter_map(Result::ok)
        .collect();

    // Vérifier si la date actuelle est > run_date + 1 jour
    let run_date = NaiveDate::parse_from_str(&attempt.run_date, "%Y-%m-%d")?;
    let now = Local::now().date_naive();

    // Le run est considéré comme "non trouvé" si la date actuelle est > run_date + 1 jour et qu'aucun dossier n'est trouvé
    // En effet, normalement, le séquenceur crée le dossier le jour même
    if now > run_date + Duration::days(1) && paths.is_empty() {
        info!(
            "Dossier non trouvé pour la tentative {} du run {}: {}. Tentative placée en erreur.",
            attempt.attempt_number, attempt.run_id, supposed_run_dir
        );
        let reason = "Le run ne se trouvait pas à l'emplacement prévu. Il peut s'agir d'une erreur dans la date, le numéro de flowcell, ou du séquenceur";
        mercure::models::analysis::fail_cannot_analyse_run(attempt.run_id, reason, pool)
            .await
            .map_err(RoutineError::from)?;
        Ok(())
    } else if paths.is_empty() {
        // Pas encore de dossier, mais on n'est pas encore le lendemain de la date du run déclarée
        info!(
            "Le run {} n'a pas encore produit de résultats. Attente.",
            attempt.run_id
        );
        Ok(())
    } else {
        let run_dir = &paths[0];
        // On démarre l'analyse
        start_analysis(&attempt, run_dir, pool).await?;

        // Seulement si l'analyse a pu être démarrée correctement, on arrive à ce point et le run peut être marqué comme en cours d'analyse
        mercure::models::analysis::start_run_analysis(attempt.run_id, pool)
            .await
            .map_err(RoutineError::from)?;
        info!(
            "Dossier trouvé pour la tentative {} du run {}: {}. Passage à l'état Running.",
            attempt.attempt_number, attempt.run_id, supposed_run_dir
        );
        Ok(())
    }
}
async fn treat_running(attempt: HgAttempt, pool: &sqlx::SqlitePool) -> Result<(), RoutineError> {
    // À implémenter
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), RoutineError> {
    use sqlx::sqlite::SqlitePool;
    use tokio::signal;

    init_logger();
    info!("Connecting to database...");
    let pool = SqlitePool::connect("sqlite://mercure.db").await?;

    // Canal pour communiquer le signal d'arrêt
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Clone le pool pour la routine
    let routine_pool = pool.clone();

    // Lancer la routine dans une tâche séparée
    let routine_handle =
        tokio::spawn(async move { run_routine_loop(routine_pool, shutdown_rx).await });

    // Attendre le signal Ctrl+C
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received, envoi du signal d'arrêt...");
            // Envoyer le signal d'arrêt à la routine
            let _ = shutdown_tx.send(true);

            // Attendre que la routine se termine proprement
            if let Err(e) = routine_handle.await {
                error!("Erreur lors de l'arrêt de la routine: {}", e);
            }

            info!("Fermeture de la connexion à la base de données...");
            pool.close().await;
            info!("Connexion à la base de données fermée. Sortie.");
        }
        Err(err) => {
            error!("Erreur lors de l'écoute du signal Ctrl+C: {}", err);
            return Err(RoutineError::SqlxError(sqlx::Error::Io(
                std::io::Error::new(std::io::ErrorKind::Other, format!("Signal error: {}", err)),
            )));
        }
    }

    Ok(())
}
