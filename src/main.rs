#[macro_use]
extern crate rocket;

mod auth;
mod db;
mod models;
mod routes;

#[launch]
async fn rocket() -> _ {
    // Initialiser la base de données
    let pool = db::init_db()
        .await
        .expect("Impossible d'initialiser la base de données");

    rocket::build()
        .manage(pool)
        .mount(
            "/",
            routes![
                // Routes HTML publiques
                routes::welcome_page,
                routes::login_page,
            ],
        )
        .mount(
            "/api",
            routes![
                // Routes publiques
                routes::public_route,
                // Routes d'authentification
                routes::login,
                routes::logout,
                // Routes protégées
                routes::me,
                routes::protected_route,
                // Routes admin
                routes::register,
                routes::register_page,
            ],
        )
}
