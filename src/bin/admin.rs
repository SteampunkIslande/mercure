use bcrypt::{DEFAULT_COST, hash};
use clap::{Parser, Subcommand};
use mercure::{
    db,
    models::user::{NewUser, User},
};
use std::{path::PathBuf, process};

#[derive(Parser)]
#[command(name = "mercure-admin")]
#[command(about = "Outil d'administration des utilisateurs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Database path
    #[arg(short, long)]
    database: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Créer un nouvel utilisateur
    CreateUser {
        /// Adrese email
        #[arg(short = 'u', long = "mail")]
        usermail: String,
        /// Nom d'utilisateur
        #[arg(short = 'n', long = "name")]
        username: String,
        /// Mot de passe
        #[arg(short = 'p', long = "password")]
        password: String,
        /// Définir comme administrateur
        #[arg(short = 'a', long = "admin")]
        admin: bool,
    },
    /// Réinitialiser le mot de passe d'un utilisateur
    ResetPassword {
        /// Nom d'utilisateur
        #[arg(short = 'u', long = "mail")]
        usermail: String,
        /// Nouveau mot de passe
        #[arg(short = 'p', long = "password")]
        password: String,
    },
    /// Rendre un utilisateur admin
    UpgradeUser {
        /// Adresse email
        #[arg(short = 'u', long = "mail")]
        usermail: String,
    },
    /// Supprimer ses droits d'administrateur à un utilisateur
    DowngradeUser {
        /// Adresse email
        #[arg(short = 'u', long = "mail")]
        usermail: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let pool = match db::init_db_from_url(&cli.database.to_string_lossy()).await {
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
            usermail,
            username,
            password,
            admin,
        } => {
            let new_user = NewUser {
                usermail,
                username,
                password,
            };
            match User::create(new_user, &pool).await {
                Ok(user) => {
                    if admin {
                        // Mettre à jour l'utilisateur comme administrateur
                        if let Err(e) = sqlx::query("UPDATE Users SET is_admin = true WHERE id = ?")
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
        Commands::ResetPassword { usermail, password } => {
            // Vérifier si l'utilisateur existe
            match User::find_by_usermail(&usermail, &pool).await {
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
                        sqlx::query("UPDATE Users SET password_hash = ? WHERE usermail = ?")
                            .bind(&password_hash)
                            .bind(&usermail)
                            .execute(&pool)
                            .await
                    {
                        eprintln!("Erreur lors de la mise à jour du mot de passe : {}", e);
                        process::exit(1);
                    }

                    println!(
                        "Mot de passe mis à jour avec succès pour l'utilisateur {}",
                        usermail
                    );
                }
                Ok(None) => {
                    eprintln!("Utilisateur non trouvé : {}", usermail);
                    process::exit(1);
                }
                Err(e) => {
                    eprintln!("Erreur lors de la recherche de l'utilisateur : {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::UpgradeUser { usermail } => {
            match User::find_by_usermail(&usermail, &pool).await {
                Ok(Some(user)) => {
                    // Mettre à jour le mot de passe
                    if let Err(e) = sqlx::query("UPDATE Users SET is_admin = 1 WHERE id = ?")
                        .bind(user.id)
                        .execute(&pool)
                        .await
                    {
                        eprintln!("Erreur lors de la mise à jour du mot de passe : {}", e);
                        process::exit(1);
                    }

                    println!(
                        "L'utilisateur {} est administrateur. Félicitations!",
                        usermail
                    );
                }
                Ok(None) => {
                    eprintln!("Utilisateur non trouvé : {}", usermail);
                    process::exit(1);
                }
                Err(e) => {
                    eprintln!("Erreur lors de la recherche de l'utilisateur : {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::DowngradeUser { usermail } => {
            match User::find_by_usermail(&usermail, &pool).await {
                Ok(Some(user)) => {
                    // Mettre à jour le mot de passe
                    if let Err(e) = sqlx::query("UPDATE Users SET is_admin = 0 WHERE id = ?")
                        .bind(user.id)
                        .execute(&pool)
                        .await
                    {
                        eprintln!("Erreur lors de la mise à jour du mot de passe : {}", e);
                        process::exit(1);
                    }

                    println!(
                        "L'utilisateur {} n'est plus administrateur. Shame!",
                        usermail
                    );
                }
                Ok(None) => {
                    eprintln!("Utilisateur non trouvé : {}", usermail);
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
