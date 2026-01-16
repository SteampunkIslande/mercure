use chrono::Local;
use env_logger::Builder;
use log::{error, info};
use mercure::models::{HgFormDef, HgRun, IndirType, ModelError};
use mercure_lib::config::get_mercure_config;
use mercure_lib::models::analysis;
use std::io::Error as IoError;
use std::process::Command;
use tokio::task::JoinError as TokioJoinError;

use sqlx::SqlitePool;
use std::fs::{Permissions, create_dir_all, exists};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use thiserror::Error;

use tokio::signal::unix::SignalKind;

use chrono::ParseError;
use mercure_lib::models::AnalysisStateMachineError;
use regex::{Error as RegexError, Regex};
use sqlx::{Error, Row};
use tokio::sync::watch;

use mercure_lib::models::HgAttempt;
use mercure_lib::models::RunStatus;
use mercure_lib::pipeline_exec::find_errors;

#[derive(Error, Debug)]
enum RoutineError {
    #[error(transparent)]
    Sqlx(#[from] Error),
    #[error(transparent)]
    AnalysisStateMachine(#[from] AnalysisStateMachineError),
    #[error(transparent)]
    DateParse(#[from] ParseError),
    #[error("{0}")]
    CustomParse(String),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    IO(#[from] IoError),
    #[error(transparent)]
    TokioJoin(#[from] TokioJoinError),
    #[error(transparent)]
    InvalidRegex(#[from] RegexError),
    #[error("Le dossier d'entrée {0} est introuvable")]
    IndirNotFound(String),
    #[error("Aucun dossier d'entrée spécifié")]
    NoIndir,
    #[error("Impossible de déterminer si le run est terminé: {0}")]
    CheckRunCompletedError(String),
}

/// Fonction de gestion du dossier des pipelines.
///
/// Argument: le chemin du dossier des pipelines
///
/// Retourne: le hash de la dernière révision (sauf si la commande échoue)
fn get_last_rev(pipeline_dir: &std::path::Path) -> Option<String> {
    String::from_utf8(
        Command::new("git")
            .current_dir(pipeline_dir)
            .arg("rev-list")
            .arg("-n")
            .arg("1")
            .arg("HEAD")
            .output()
            .ok()?
            .stdout,
    )
    .ok()
}

/// Fonction de gestion du dossier des pipelines.
///
/// Argument: le chemin du dossier des pipelines
///
/// Pas de retour: effet de bord. Met à jour le dossier des pipelines
fn git_pull(pipeline_dir: &std::path::Path) {
    let pull_output = Command::new("git")
        .arg("pull")
        .arg("-C")
        .arg(pipeline_dir)
        .output();

    match pull_output {
        Err(e) => {
            error!(
                "Erreur lors de l'exécution de 'git pull' dans {}: {}",
                pipeline_dir.display(),
                e
            );
        }
        Ok(output) => {
            if let Some(code) = output.status.code() {
                if code != 0 {
                    error!("{}", String::from_utf8_lossy(output.stdout.as_ref()));
                }
            }
        }
    }
}

/// Fonction de gestion du dossier des pipelines.
///
/// Argument: le chemin du dossier des pipelines
///
/// Pas de retour: effet de bord. Permet de s'assurer que le dossier de pipeline se retrouve read-only
fn set_read_only(pipeline_dir: &std::path::Path) {
    let pull_output = Command::new("chmod")
        .arg("-R")
        .arg("a-w")
        .arg(pipeline_dir)
        .output();

    match pull_output {
        Err(e) => {
            error!(
                "Erreur lors de l'exécution de `chmod -R a-w {}`:\n{}",
                pipeline_dir.display(),
                e
            );
        }
        Ok(output) => {
            if let Some(code) = output.status.code() {
                if code != 0 {
                    error!(
                        "`chmod -R a-w {}`:\n{}",
                        pipeline_dir.display(),
                        String::from_utf8_lossy(output.stdout.as_ref())
                    );
                }
            }
        }
    }
}

/// Fonction qui exécute la boucle de routine avec des points de contrôle pour l'annulation
async fn run_routine_loop(pool: sqlx::SqlitePool, mut shutdown_rx: watch::Receiver<bool>) {
    use std::time::Duration;

    loop {
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
                    let error_str = if matches!(e, RoutineError::IndirNotFound(_)) {
                        format!(
                            "{}. Il peut s'agir d'une erreur dans la date, le numéro de flowcell, ou du séquenceur.",
                            e
                        )
                    } else {
                        e.to_string()
                    };
                    match analysis::fail_cannot_analyse_run(attempt.run_id, &error_str, &pool).await
                    {
                        Ok(_) => info!(
                            "La tentative {} du run {} a été marquée comme Failed.",
                            attempt.attempt_number, attempt.run_id
                        ),
                        Err(err) => error!(
                            "Erreur lors du marquage du run {}, tentative {} comme Failed:\n {}",
                            attempt.run_id, attempt.attempt_number, err
                        ),
                    }
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

        // Compter les runs avec le statut Pending. S'il n'y en a aucun, alors on peut lancer `git pull` pour mettre à jour les pipelines
        let pending_count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM Attempts WHERE status = ?")
                .bind(RunStatus::Pending.to_string())
                .fetch_one(&pool)
                .await
                .unwrap_or(0);
        if pending_count == 0 {
            let config = get_mercure_config();
            let pipeline_dir = Path::new(&config.pipeline_dir);
            let rev_before = get_last_rev(pipeline_dir);
            git_pull(pipeline_dir);
            set_read_only(pipeline_dir);
            let rev_after = get_last_rev(pipeline_dir);
            if rev_before != rev_after {
                info!("Pipelines mis à jour avec succès !");
            }
        }

        // Point de contrôle 2: Attendre avec possibilité d'interruption
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(60)) => {},
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() { eprintln!("Signal d'arrêt reçu, arrêt de la routine...");break; }
            }
        }
    }
}

fn get_indir_outdir_for_analysisdir(
    attempt: &HgAttempt,
) -> Result<(PathBuf, PathBuf), RoutineError> {
    let indir_str = attempt.indir.as_ref().ok_or(RoutineError::NoIndir)?;
    Ok((PathBuf::from(&indir_str), PathBuf::from(&indir_str)))
}

fn get_indir_outdir_for_ontdir(
    attempt: &HgAttempt,
    run: &HgRun,
) -> Result<(PathBuf, PathBuf), RoutineError> {
    let indir_str = attempt.indir.as_ref().ok_or(RoutineError::NoIndir)?;

    let config = get_mercure_config();

    let run_date_short = attempt.run_date[2..].replace("-", "");
    let seq_name = &attempt.run_sequencer;
    let run_name = &run.run_name;

    Ok((
        PathBuf::from(&indir_str),
        PathBuf::from(config.analysis_dir).join(format!("{run_date_short}_{seq_name}_{run_name}")),
    ))
}

fn get_indir_outdir_for_illumina(
    attempt: &HgAttempt,
    _run: &HgRun,
) -> Result<(PathBuf, PathBuf), RoutineError> {
    // Le dossier de run brut Illumina devrait être dans {sequencers_dir}/{run_sequencer}/output/{run_date}_{run_sequencer}_*_{run_flowcellid}*
    let config = get_mercure_config();
    let raw_dir = Path::new(&config.sequencers_dir)
        .join(&attempt.run_sequencer)
        .join("output");

    let run_date_short = attempt.run_date[2..].replace("-", "");
    let seq_name = &attempt.run_sequencer;
    let flowcell_id = &attempt.run_flowcellid;

    let run_dir_pattern = format!(r"^{run_date_short}_{seq_name}_(\d+)_.+{flowcell_id}$");
    let run_dir_re = Regex::new(&run_dir_pattern)?;

    let (bcl_dir_base, seq_run_counter) = std::fs::read_dir(&raw_dir)?
        .filter_map(|e| {
            e.map(|e| {
                run_dir_re
                    .captures(&e.file_name().display().to_string())
                    .and_then(|cap| {
                        cap.get(1).and_then(|c| {
                            Some((
                                e.file_name().display().to_string(),
                                c.as_str().to_string().parse::<i64>().ok()?,
                            ))
                        })
                    })
            })
            .ok()?
        })
        .next()
        .ok_or_else(|| RoutineError::IndirNotFound(run_dir_pattern))?;

    let output_dir = PathBuf::from(&config.analysis_dir).join(format!(
        "{}_{}_{}",
        run_date_short, seq_name, seq_run_counter
    ));

    Ok((raw_dir.join(bcl_dir_base), output_dir))
}

/// Cette fonction crée un fichier tel que spécifié dans le formulaire
/// C'est HgRun qui a un membre dédié
async fn start_analysis(
    attempt: &HgAttempt,
    _form: &HgFormDef,
    input_dir: &Path,
    output_dir: &Path,
    pool: &sqlx::SqlitePool,
) -> Result<(), RoutineError> {
    let config = get_mercure_config();

    // En mode reproduction, le dossier de pipeline sera cloné dans un dossier temporaire avec le commit préalablement enregistré dans la tentative
    let pipeline_base_dir = Path::new(&config.pipeline_dir);

    // Création du dossier d'analyse si nécessaire, *avant* de copier les fichiers!
    if !output_dir.exists() {
        if let Err(e) = create_dir_all(output_dir) {
            error!("Erreur lors de la création du dossier d'analyse: {}", e);
            return Err(e.into());
        }
        info!("Dossier d'analyse créé: {}", output_dir.display());
    }

    //Copie des fichiers adn.csv, arn.csv et metadata si présents
    if std::path::Path::new(&attempt.sample_sheet_adn_path).exists() {
        let dest_adn_path = input_dir.join("adn.csv");
        std::fs::copy(&attempt.sample_sheet_adn_path, &dest_adn_path)?;
        std::fs::set_permissions(&dest_adn_path, Permissions::from_mode(0o644))?;

        info!(
            "Fichier ADN copié de {} vers {}",
            &attempt.sample_sheet_adn_path,
            dest_adn_path.display()
        );
    }
    if std::path::Path::new(&attempt.sample_sheet_arn_path).exists() {
        let dest_arn_path = input_dir.join("arn.csv");
        std::fs::copy(&attempt.sample_sheet_arn_path, &dest_arn_path)?;
        std::fs::set_permissions(&dest_arn_path, Permissions::from_mode(0o644))?;

        info!(
            "Fichier ARN copié de {} vers {}",
            &attempt.sample_sheet_arn_path,
            dest_arn_path.display()
        );
    }
    if std::path::Path::new(&attempt.metadata_path).exists()
        && let Some(src_metadata_filename) =
            std::path::Path::new(&attempt.metadata_path).file_name()
    {
        let dest_metadata_path = input_dir.join(src_metadata_filename);
        std::fs::copy(&attempt.metadata_path, &dest_metadata_path)?;
        std::fs::set_permissions(&dest_metadata_path, Permissions::from_mode(0o644))?;

        info!(
            "Fichier Metadata copié de {} vers {}",
            &attempt.metadata_path,
            dest_metadata_path.display()
        );
    }
    // Génération du script d'analyse
    generate_script(input_dir, output_dir, pipeline_base_dir, attempt, pool).await?;

    Ok(())
}

/// Fonction pour trouver les runs en attente
///
/// Quelques effets de bord :
/// - Log les erreurs SQL
/// - Log les erreurs de récupération des tentatives
///
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
///
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

/// Génère le script d'analyse pour une tentative donnée
async fn generate_script(
    input_dir: &Path,
    output_dir: &Path,
    pipeline_base_dir: &Path,
    attempt: &HgAttempt,
    pool: &SqlitePool,
) -> Result<(), RoutineError> {
    let config = mercure_lib::config::get_mercure_config();

    // Obtention des informations sur le pipeline et le launcher choisis
    let run: HgRun = HgRun::get_run_from_id(attempt.run_id, pool).await?;

    // Le chemin du launcher doit être absolu. C'est ce chemin qui va se trouver dans le script généré
    let launcher_abs_path = format!(
        "{base}/{pipeline}/launchers/{launcher}",
        base = pipeline_base_dir.display(),
        pipeline = run.form.pipeline_name,
        launcher = run.form.launcher_name
    );
    if !exists(&launcher_abs_path)? {
        return Err(IoError::new(
            std::io::ErrorKind::NotFound,
            format!("Le launcher {} est introuvable!", &launcher_abs_path),
        )
        .into());
    }

    let script_path = format!(
        "{jobs_dir}/TODO/job-{run_id}-{attempt_number}.sh",
        jobs_dir = config.jobs_dir,
        run_id = attempt.run_id,
        attempt_number = attempt.attempt_number
    );
    if exists(&script_path)? {
        return Err(IoError::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Le fichier {} existe déjà!", &script_path),
        )
        .into());
    }

    let exported_vars = attempt
        .user_defined_vars
        .iter()
        .map(|(k, v)| format!(r#"export {k}="{v}""#))
        .chain([
            format!(r#"export INDIR="{}""#, input_dir.display()),
            format!(r#"export OUTDIR="{}""#, output_dir.display()),
            format!(r#"export RUN_NAME="{}""#, &run.run_name),
        ])
        .collect::<Vec<_>>()
        .join("\n");

    let mut script_file = std::fs::OpenOptions::new()
        .mode(0o775)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&script_path)?;

    // Un launcher peut contenir une ligne shebang (#!). Cette ligne n'est pertinente qu'en standalone.
    // Dans notre cas, on veut exécuter le launcher dans le contexte du script généré, donc on ignore cette ligne.
    let launcher_content = std::fs::read_to_string(&launcher_abs_path)?
        .split('\n')
        .filter(|s| !s.starts_with("#!"))
        .collect::<Vec<&str>>()
        .join("\n");

    write!(
        &mut script_file,
        r#"#!/bin/bash

export PIPELINE_DIR="{pipeline_dir}"
export PIPELINE_NAME="{pipeline_name}"
export PATH=/usr/bin:$PATH
source /etc/profile

{udv}

{launcher_content}

"#,
        pipeline_dir = pipeline_base_dir.display(),
        pipeline_name = run.form.pipeline_name,
        udv = exported_vars
    )?;

    Ok(())
}

fn check_illumina_run_completion(indir: &PathBuf, seq_name: &str) -> Result<bool, RoutineError> {
    let config = get_mercure_config();

    let output = Command::new(&config.check_run_completed)
        .arg(indir)
        .arg(seq_name)
        .output()
        .map_err(|e| RoutineError::CheckRunCompletedError(format!("{e}")))?;
    Ok(output.status.success())
}

/// Traite un run en état Pending (décide ou non de lancer l'analyse automatiquement)
///
/// Si indir_type == BclDir, cherche le dossier de run et démarre l'analyse si trouvé et que le contenu du dossier reflète que le run est terminé.
/// Si indir_type == AnalysisDir, démarre directement l'analyse dans le dossier spécifié (indir = outdir = analysis_dir)
/// Si indir_type == OntDir, démarre directement l'analyse. outdir = /data/analysis/{run_date}_{run_sequencer}_{RUN_NAME (sans espaces)}
async fn treat_pending(attempt: &HgAttempt, pool: &sqlx::SqlitePool) -> Result<(), RoutineError> {
    use chrono::{Duration, Local, NaiveDate};

    let run: HgRun = HgRun::get_run_from_id(attempt.run_id, pool).await?;
    let form: &HgFormDef = &run.form;

    // Vérifier si la date actuelle est > run_date + 1 jour
    let run_date = NaiveDate::parse_from_str(&attempt.run_date, "%Y-%m-%d")?;
    let now = Local::now().date_naive();

    let (input_dir, output_dir) = match form.indir_type {
        IndirType::BclDir => {
            match get_indir_outdir_for_illumina(attempt, &run) {
                Err(RoutineError::IndirNotFound(expected_name)) => {
                    // Le run est considéré comme "non trouvé" si la date actuelle est > run_date + 1 jour et qu'aucun dossier n'est trouvé
                    // En effet, normalement, le séquenceur crée le dossier le jour même
                    if now > run_date + Duration::days(1) {
                        info!(
                            "Dossier non trouvé pour la tentative {} du run {}: Nom du dossier attendu: {}.",
                            attempt.attempt_number, attempt.run_id, expected_name
                        );
                        return Err(RoutineError::IndirNotFound(expected_name));
                    } else {
                        // Rien d'alarmant au fait que le dossier de run soit introuvable. Le run n'a peut-être pas encore été lancé.
                        return Ok(());
                    }
                }
                Err(RoutineError::InvalidRegex(e)) => {
                    error!("Regex invalide: {}", e);
                    return Err(RoutineError::InvalidRegex(e));
                }
                Err(e) => {
                    error!("Erreur générique: {}", e);
                    return Err(e);
                }
                Ok((in_dir, out_dir)) => (in_dir, out_dir),
            }
        }
        IndirType::AnalysisDir => get_indir_outdir_for_analysisdir(attempt)?,
        IndirType::OntDir => get_indir_outdir_for_ontdir(attempt, &run)?,
    };

    // Vérifier le contenu du dossier d'entrée pour s'assurer que le run est terminé (uniquement pour Illumina/BclDir)
    // Si le run n'est pas terminé, on retourne prématurément avec un résultat OK
    if matches!(form.indir_type, IndirType::BclDir)
        && !check_illumina_run_completion(&input_dir, &attempt.run_sequencer)?
    {
        return Ok(());
    }

    // On démarre l'analyse
    start_analysis(attempt, form, &input_dir, &output_dir, pool).await?;

    // Seulement si l'analyse a pu être démarrée correctement, on arrive à ce point et le run peut être marqué comme en cours d'analyse
    mercure::models::analysis::start_run_analysis(attempt.run_id, pool)
        .await
        .map_err(RoutineError::from)?;
    info!(
        "Dossier trouvé pour la tentative {} du run {}: {}. Passage à l'état Running.",
        attempt.attempt_number,
        attempt.run_id,
        input_dir.display()
    );
    Ok(())
}

fn post_run_command(attempt: &HgAttempt, log_file_name: &PathBuf, status: &str) {
    let config = get_mercure_config();
    match {
        let mut cmd = Command::new(&config.post_run_script);
        cmd.env("HG_LOG_FILE", log_file_name.with_extension("log"))
            .env("OUTDIR", attempt.outdir.as_ref().unwrap_or(&"".to_string()))
            .env("INDIR", attempt.indir.as_ref().unwrap_or(&"".to_string()))
            .env("HG_ATTEMPT_ID", attempt.attempt_number.to_string())
            .env("HG_RUN_ID", attempt.run_id.to_string())
            .env("HG_ATTEMPT_STATUS", status);
        // Définir les variables d'environnement du run pour l'exécution de post-run-script
        for udv in attempt.user_defined_vars.iter() {
            cmd.env(udv.0, udv.1);
        }
        cmd
    }
    .output()
    {
        Ok(output) => {
            if output.status.success() {
                info!("Script post-run exécuté avec succès.")
            } else {
                error!(
                    "Le script post-run a rencontré une erreur (code {}), message:\n{}",
                    output.status.code().unwrap_or_default(),
                    String::from_utf8(
                        output
                            .stdout
                            .into_iter()
                            .chain(output.stderr.into_iter())
                            .collect()
                    )
                    .unwrap_or("Impossible de capturer la sortie, non UTF-8 !".to_string())
                )
            }
        }
        Err(e) => {
            error!("Erreur lors de l'exécution du script post-run:\n{e}")
        }
    }
}

async fn treat_running(attempt: HgAttempt, pool: &sqlx::SqlitePool) -> Result<(), RoutineError> {
    let config = get_mercure_config();

    let running_dir = format!("{}/RUNNING", config.jobs_dir);
    let fails_dir = format!("{}/FAILS", config.jobs_dir);
    let done_dir = format!("{}/DONE", config.jobs_dir);

    let file_pattern = format!("job-{}-{}.sh", attempt.run_id, attempt.attempt_number);

    let fails_paths: Vec<PathBuf> = std::fs::read_dir(&fails_dir)?
        .map(|entry| entry.map(|e| e.path()))
        .filter_map(Result::ok)
        .filter(|path| {
            path.file_name()
                .map(|name| name.display().to_string().ends_with(&file_pattern))
                .unwrap_or(false)
        })
        .collect();

    let done_paths: Vec<PathBuf> = std::fs::read_dir(&done_dir)?
        .map(|entry| entry.map(|e| e.path()))
        .filter_map(Result::ok)
        .filter(|path| {
            path.file_name()
                .map(|name| name.display().to_string().ends_with(&file_pattern))
                .unwrap_or(false)
        })
        .collect();
    let running_paths: Vec<PathBuf> = std::fs::read_dir(&running_dir)?
        .map(|entry| entry.map(|e| e.path()))
        .filter_map(Result::ok)
        .filter(|path| {
            path.file_name()
                .map(|name| name.display().to_string().ends_with(&file_pattern))
                .unwrap_or(false)
        })
        .collect();

    if running_paths.len() + fails_paths.len() + done_paths.len() > 1 {
        Err(RoutineError::CustomParse(format!(
            "La tentative {} pour le run {} apparaît dans plusieurs états à la fois",
            attempt.attempt_number, attempt.run_id
        )))
    } else {
        if running_paths.len() == 1 {
            // Le run est toujours en cours d'analyse
            // info!(
            //     "La tentative {} du run {} n'est pas encore terminée.",
            //     attempt.attempt_number, attempt.run_id
            // );
        }
        if fails_paths.len() == 1 {
            info!(
                "La tentative {} du run {} s'est terminée avec une erreur. Exécution du script post-run...",
                attempt.attempt_number, attempt.run_id
            );
            post_run_command(&attempt, &fails_paths[0], "FAILED");
            //Renommer le dossier de sortie avec le suffixe `-failed-{run_id}-{attempt_id}`
            if let Some(outdir) = attempt.outdir.as_ref() {
                let dest = format!(
                    "{outdir}-failed-{}-{}",
                    attempt.run_id, attempt.attempt_number
                );
                if !std::path::PathBuf::from(&dest).exists() {
                    info!("Renommage de `{}` vers `{}`", outdir, dest);
                    std::fs::rename(outdir, dest)?;
                }
            }
            // grep '## ERROR' dans le fichier de log principal pour obtenir la raison de l'échec

            let error_list = find_errors(
                attempt.run_id,
                attempt.attempt_number,
                PathBuf::from(&config.logs_dir),
            )
            .await
            .unwrap_or_default();

            let error_reason = error_list.last().cloned().unwrap_or_else(|| {
                "Erreur inconnue lors de l'analyse. Voir les logs pour plus de détails.".to_string()
            });
            analysis::complete_failure(attempt.run_id, &error_reason, pool).await?;
            analysis::comment_attempt(
                attempt.run_id,
                attempt.attempt_number,
                &error_list.join("\n"),
                pool,
            )
            .await?;
        }
        if done_paths.len() == 1 {
            info!(
                "La tentative {} du run {} s'est terminée avec succès. Exécution du script post-run...",
                attempt.attempt_number, attempt.run_id
            );
            post_run_command(&attempt, &done_paths[0], "SUCCESS");
            analysis::complete_success(attempt.run_id, pool).await?;
        }
        Ok(())
    }
}

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

#[tokio::main]
async fn main() -> Result<(), RoutineError> {
    use sqlx::sqlite::SqlitePool;

    init_logger();
    info!("Connecting to database...");
    let config = get_mercure_config();
    let pool = SqlitePool::connect(&config.mercure_db).await?;

    // Canal pour communiquer le signal d'arrêt
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Clone le pool pour la routine
    let routine_pool = pool.clone();

    // Lancer la routine dans une tâche séparée
    let routine_handle =
        tokio::spawn(async move { run_routine_loop(routine_pool, shutdown_rx).await });

    // Attendre le signal SIGTERM
    // Attendre aussi le signal SIGINT (Ctrl+C) pour permettre un arrêt propre lors du développement local
    let mut stream_sigterm = tokio::signal::unix::signal(SignalKind::terminate())?;
    let mut stream_sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;

    tokio::select! {
        _ = stream_sigterm.recv() => {
            info!("Signal SIGTERM reçu, arrêt de la routine...");
            let _ = shutdown_tx.send(true);
            routine_handle.await?;
        }
        _ = stream_sigint.recv() => {
            info!("Signal SIGINT reçu, arrêt de la routine...");
            let _ = shutdown_tx.send(true);
            routine_handle.await?;
        }
    }

    info!("Routine arrêtée.");
    Ok(())
}
