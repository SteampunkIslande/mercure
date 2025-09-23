use rocket::fs::NamedFile;

#[macro_use]
extern crate rocket;

mod auth;
mod db;
mod models;
mod routes;

#[catch(401)]
pub async fn unauthorized() -> Option<NamedFile> {
    NamedFile::open("templates/unauthorized.html").await.ok()
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
        .mount(
            "/mercure",
            routes![
                // Routes génériques: Gestion de l'authentification
                routes::welcome_page,
                routes::login_get,
                routes::logout,
            ],
        )
        .mount(
            "/api",
            routes![
                // Routes admin
                routes::register,
                routes::register_page,
                routes::admin_landing_page,
                // Route login de l'API
                routes::login_post
            ],
        )
}
