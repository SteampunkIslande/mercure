use std::collections::HashSet;
use std::net::Ipv4Addr;

use clap::Parser;
use log::info;
use mercure_lib::templates::{Template, context, minijinja_fairing};
use rocket::Build;
use rocket::Config;
use rocket::Rocket;
use rocket::config::Shutdown;
use rocket::fs::FileServer;
use tokio::signal::unix::{SignalKind, signal};

use rocket::{catch, catchers, routes};

use anyhow::Result;
use mercure_lib::config;
use mercure_lib::routes;
use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::routine::run_routine_loop;

#[catch(401)]
async fn unauthorized() -> Template {
    Template::render(
        "errors/unauthorized",
        context! {
            title=> "Session expirée",
            h2=> "Session expirée",
            message=> "Votre session a expirée, veuillez vous reconnecter"
        },
    )
}

async fn rocket(pool: SqlitePool, host: Option<Ipv4Addr>, port: Option<u16>) -> Rocket<Build> {
    let config = config::get_mercure_config();

    let shutdown_config = Shutdown {
        ctrlc: false,
        signals: HashSet::new(),
        ..Default::default()
    };

    let mut figment = Config::figment().merge(("shutdown", shutdown_config));
    if let Some(host) = host {
        figment = figment.merge(("address", host));
    }
    if let Some(port) = port {
        figment = figment.merge(("port", port));
    }

    rocket::custom(figment)
        .register("/mercure", catchers![unauthorized])
        .mount("/mercure/static", FileServer::from(config.static_dir))
        .mount("/mercure/uploads", FileServer::from(config.upload_dir))
        .mount("/mercure/logs", FileServer::from(config.logs_dir))
        .mount(
            "/mercure",
            routes![
                // Page d'accueil
                routes::frontend::welcome_page_get,
                // Page de login
                routes::frontend::login_get,
                // Simple GET pour se déconnecter, la seule route GET qui appartient au backend
                routes::backend::logout_get,
                // Routes pour le frontend: renvoie toujours du HTML
                routes::frontend::home_get,
                routes::frontend::password_edit_get,
                routes::frontend::new_run_get,
                // Affichage d'un run spécifique
                routes::frontend::show_run_get,
                // Affichage de la liste des runs
                routes::frontend::list_runs,
                // Recherche avancée de runs
                routes::frontend::search_run,
                // Edition d'un run
                routes::frontend::edit_run_get
            ],
        )
        .mount(
            "/mercure/admin",
            //Routes réservées à l'admin
            routes![
                routes::frontend::register_get,
                routes::frontend::admin_dashboard_get,
                routes::frontend::newform_get,
                routes::frontend::editform_get,
                routes::frontend::show_forms_get,
                routes::frontend::edit_users,
                routes::frontend::edit_groups_for_user,
                routes::frontend::edit_groups_get,
                // Route pour gérer les mises à jour des formulaires
                routes::frontend::forms_list_updates_get,
            ],
        )
        .mount(
            "/mercure/api",
            routes![
                // Routes pour le backend: renvoie toujours du JSON
                routes::backend::register_post,
                routes::backend::login_post,
                routes::backend::newform_post,
                routes::backend::newgroup_get,
                routes::backend::list_groups,
                routes::backend::get_form_from_id,
                routes::backend::get_all_forms_for_group,
                // Simple GET pour mettre un formulaire en production
                routes::backend::enable_form,
                // Simple GET pour retirer un formulaire du service
                routes::backend::disable_form,
                // Simple GET pour lister les utilisateurs
                routes::backend::list_users,
                // Simple GET pour lister les groupes d'un utilisateur
                routes::backend::list_groups_for_user,
                routes::backend::update_groups,
                routes::backend::get_all_forms,
                routes::backend::password_edit_post,
                routes::backend::newrun_post,
                routes::backend::editrun_post,
                routes::backend::validate_run_post,
                routes::backend::upload_post,
                routes::backend::list_runs_get,
                routes::backend::check_samplesheet,
                routes::backend::list_samples_from_samplesheet,
                routes::backend::parse_launcher_endpoint,
                routes::backend::edit_form_groups,
                routes::backend::get_nextversion,
                // Route renvoyant un EventStream
                routes::backend::watch,
                // Route pour rechercher des runs
                routes::backend::search_run_get,
                // Route pour traduire les noms des séquenceurs
                routes::backend::prettify_seqname,
                // Routes pour lister les dossiers selon le type
                routes::backend::list_directories_by_type,
                // Route pour toutes les redirections
                routes::frontend::redirect_get,
                // Route pour relancer un run (une fois terminé)
                routes::backend::retry_run_get,
                // Route pour mettre à jour la révision d'un formulaire
                routes::backend::forms_list_updates_get,
                // Route pour obtenir le statut de tous les formulaires avec pagination
                routes::backend::forms_update_status_paginated_get
            ],
        )
        .manage(pool)
        .attach(minijinja_fairing())
}

#[derive(Parser)]
pub struct Web {
    /// L'adresse IP sur laquelle écouter
    #[arg(short = 'H', long = "host")]
    ip: Option<Ipv4Addr>,

    #[arg(short = 'p', long = "port")]
    port: Option<u16>,
}

impl Web {
    pub async fn run(self, pool: SqlitePool) -> Result<()> {
        let rocket_app = rocket(pool.clone(), self.ip, self.port)
            .await
            .ignite()
            .await?;
        let shutdown_handle = rocket_app.shutdown();

        // Canal pour prévenir la routine qu'elle doit s'arrêter
        let (routine_shutdown_tx, routine_shutdown_rx) = broadcast::channel(1);

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Impossible d'écouter le signal SIGTERM");
        let mut sigint =
            signal(SignalKind::interrupt()).expect("Impossible d'écouter le signal SIGINT");
        let mut sigquit =
            signal(SignalKind::quit()).expect("Impossible d'écouter le signal SIGQUIT");
        // On lance une tâche dédiée à l'écoute des signaux OS
        tokio::spawn(async move {
            tokio::select! {
                _ = sigterm.recv() => info!("SIGTERM reçu."),
                _ = sigint.recv() => info!("SIGINT (Ctrl+C) reçu."),
                _ = sigquit.recv() => info!("SIGQUIT reçu."),
            }
            // Notification de l'arrêt au runtime de Rocket
            shutdown_handle.notify();

            // Envoi d'un signal de shutdown à la routine
            let _ = routine_shutdown_tx.send(());
        });

        // Démarrage de la routine (exécution des jobs)
        let routine = run_routine_loop(pool, routine_shutdown_rx);

        // Démarrage de l'application web
        let webapp = rocket_app.launch();

        // Attendre que l'application web et la routine sont tous les deux terminés
        let (web_res, routine_res) = tokio::join!(webapp, routine);

        if let Err(e) = web_res {
            log::error!("Erreur de fermeture Rocket: {}", e);
        }
        if let Err(e) = routine_res {
            log::error!("Erreur de fermeture Routine: {}", e);
        }

        Ok(())
    }
}
