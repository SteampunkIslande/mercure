use rocket::fs::FileServer;
use rocket_dyn_templates::minijinja::UndefinedBehavior;
use rocket_dyn_templates::{Template, context};

#[macro_use]
extern crate rocket;

use mercure_lib::config;
use mercure_lib::db;
use mercure_lib::routes;

#[catch(401)]
pub async fn unauthorized() -> Template {
    Template::render(
        "errors/unauthorized",
        context! {
            title: "Session expirée",
            h2: "Session expirée",
            message: "Votre session a expirée, veuillez vous reconnecter"
        },
    )
}

#[launch]
async fn rocket() -> _ {
    // Initialiser la base de données
    let pool = db::init_db()
        .await
        .expect("Impossible d'initialiser la base de données");

    let config = config::get_mercure_config();

    rocket::build()
        .register("/", catchers![unauthorized])
        .mount("/static", FileServer::from(config.static_dir))
        .mount("/uploads", FileServer::from(config.upload_dir))
        .mount("/logs", FileServer::from(config.logs_dir))
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
                routes::frontend::edit_groups_get
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
                routes::backend::redirect_post,
                // Route pour relancer un run (une fois terminé)
                routes::backend::retry_run_get,
                // Route pour commenter une tentative
                routes::backend::update_comment
            ],
        )
        .manage(pool)
        .attach(Template::custom(|engines| {
            engines
                .minijinja
                .set_undefined_behavior(UndefinedBehavior::SemiStrict);
        }))
}
