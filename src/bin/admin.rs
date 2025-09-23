use bcrypt::{DEFAULT_COST, hash};
use clap::{Parser, Subcommand};
use mercure::{
    db,
    models::user::{NewUser, User},
};
use std::process;

#[derive(Parser)]
#[command(name = "mercure-admin")]
#[command(about = "Outil d'administration des utilisateurs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Créer un nouvel utilisateur
    CreateUser {
        /// Nom d'utilisateur
        #[arg(short, long)]
        username: String,
        /// Mot de passe
        #[arg(short, long)]
        password: String,
        /// Définir comme administrateur
        #[arg(short, long)]
        admin: bool,
    },
    /// Réinitialiser le mot de passe d'un utilisateur
    ResetPassword {
        /// Nom d'utilisateur
        #[arg(short, long)]
        username: String,
        /// Nouveau mot de passe
        #[arg(short, long)]
        password: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let pool = match db::init_db().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!(
                "Erreur lors de l'initialisation de la base de données : {}",
                e
            );
            process::exit(1);
        }
    };

    match cli.command {
        Commands::CreateUser {
            username,
            password,
            admin,
        } => {
            let new_user = NewUser { username, password };
            match User::create(new_user, &pool).await {
                Ok(user) => {
                    if admin {
                        // Mettre à jour l'utilisateur comme administrateur
                        if let Err(e) = sqlx::query("UPDATE users SET is_admin = true WHERE id = ?")
                            .bind(user.id)
                            .execute(&pool)
                            .await
                        {
                            eprintln!(
                                "Erreur lors de la définition des droits administrateur : {}",
                                e
                            );
                            process::exit(1);
                        }
                    }
                    println!(
                        "Utilisateur créé avec succès{}",
                        if admin { " (administrateur)" } else { "" }
                    );
                }
                Err(e) => {
                    eprintln!("Erreur lors de la création de l'utilisateur : {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::ResetPassword { username, password } => {
            // Vérifier si l'utilisateur existe
            match User::find_by_username(&username, &pool).await {
                Ok(Some(_)) => {
                    // Hasher le nouveau mot de passe
                    let password_hash = match hash(password.as_bytes(), DEFAULT_COST) {
                        Ok(hash) => hash,
                        Err(e) => {
                            eprintln!("Erreur lors du hashage du mot de passe : {}", e);
                            process::exit(1);
                        }
                    };

                    // Mettre à jour le mot de passe
                    if let Err(e) =
                        sqlx::query("UPDATE users SET password_hash = ? WHERE username = ?")
                            .bind(&password_hash)
                            .bind(&username)
                            .execute(&pool)
                            .await
                    {
                        eprintln!("Erreur lors de la mise à jour du mot de passe : {}", e);
                        process::exit(1);
                    }

                    println!(
                        "Mot de passe mis à jour avec succès pour l'utilisateur {}",
                        username
                    );
                }
                Ok(None) => {
                    eprintln!("Utilisateur non trouvé : {}", username);
                    process::exit(1);
                }
                Err(e) => {
                    eprintln!("Erreur lors de la recherche de l'utilisateur : {}", e);
                    process::exit(1);
                }
            }
        }
    }
}
