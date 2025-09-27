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
    // Initialiser la base de données
    let pool = db::init_db()
        .await
        .expect("Impossible d'initialiser la base de données");

    rocket::build()
        .manage(pool)
        .register("/", catchers![unauthorized])
        .mount("/static", FileServer::from("./static"))
        .mount(
            "/mercure",
            routes![
                // Routes génériques: accueil et authentification
                routes::welcome_page_get,
                routes::login_get,
                routes::logout_get,
                // Création de formulaire
                routes::newform_get
            ],
        )
        .mount(
            "/mercure/admin",
            routes![
                // Routes pour affichage dans le navigateur de l'admin
                routes::register_get,
                routes::admin_landing_page_get
            ],
        )
        .mount(
            "/mercure/api",
            routes![
                // Routes pour le backend: renvoie toujours du JSON
                routes::register_post,
                routes::login_post,
                routes::newform_post,
                routes::newgroup_get,
                routes::list_groups
            ],
        )
        .attach(Template::fairing())
}
