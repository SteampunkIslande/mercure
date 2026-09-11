use anyhow::{Result, bail};
use chrono::Local;
use env_logger::Builder;
use mercure::config::get_mercure_config;
use std::io::Write;

mod admin;
mod routine;
mod web;
use clap::{Parser, Subcommand};
use mercure::db;

use crate::admin::Admin;
use crate::web::Web;

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

#[derive(Parser)]
#[command(about = "Application web dédiée au lancement des analyses bio-informatiques", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Database path
    #[arg(short, long)]
    database: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(subcommand)]
    Admin(Admin),

    Web(Web),
}

#[rocket::main]
async fn main() -> Result<()> {
    init_logger();

    let cli = Cli::parse();

    let database_path = cli.database.unwrap_or(get_mercure_config().mercure_db);

    let pool = match db::init_db_from_url(&database_path).await {
        Ok(pool) => pool,
        Err(e) => {
            bail!(
                "Erreur lors de l'initialisation de la base de données : {}",
                e
            );
        }
    };

    match cli.command {
        Commands::Admin(admin) => admin.run(pool).await?,
        Commands::Web(web) => web.run(pool).await?,
    }
    Ok(())
}
