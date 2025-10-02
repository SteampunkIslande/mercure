use rocket::fs::{FileServer, NamedFile};
use rocket_dyn_templates::Template;

#[macro_use]
extern crate rocket;

mod auth;
mod db;
mod models;
mod routes;

#[catch(401)]
pub async fn unauthorized() -> Option<NamedFile> {
    NamedFile::open("static/errors/unauthorized.html")
        .await
        .ok()
}

#[launch]
async fn rocket() -> _ {
    let rocket_app = rocket::build()
        .register("/", catchers![unauthorized])
        .mount("/static", FileServer::from("./static"))
        .mount(
            "/mercure",
            routes![
                routes::frontend::welcome_page_get,
                routes::frontend::login_get,
                routes::backend::logout_get,
                routes::frontend::success_page_get,
                routes::frontend::home_get,
                routes::frontend::password_edit_get,
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
                routes::backend::list_users,
                routes::backend::list_groups_for_user,
                routes::backend::update_groups,
                routes::backend::get_all_forms,
                routes::backend::password_edit_post
            ],
        )
        .attach(Template::fairing());

    // Initialiser la base de données
    let pool = db::init_db(&rocket_app)
        .await
        .expect("Impossible d'initialiser la base de données");

    rocket_app.manage(pool)
}
