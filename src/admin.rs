use bcrypt::{DEFAULT_COST, hash};
use clap::Subcommand;
use smaug::models::user::{NewUser, User};
use sqlx::SqlitePool;

use std::process;

use anyhow::{Result, bail};

#[derive(Subcommand)]
pub enum Admin {
    /// Create a new user
    CreateUser {
        /// Email address
        #[arg(short = 'u', long = "mail")]
        usermail: String,
        /// Username
        #[arg(short = 'n', long = "name")]
        username: String,
        /// Password
        #[arg(short = 'p', long = "password")]
        password: String,
        /// Set as administrator
        #[arg(short = 'a', long = "admin")]
        admin: bool,
    },
    /// Reset a user's password
    ResetPassword {
        /// Username
        #[arg(short = 'u', long = "mail")]
        usermail: String,
        /// New password
        #[arg(short = 'p', long = "password")]
        password: String,
    },
    /// Make a user an admin
    UpgradeUser {
        /// Email address
        #[arg(short = 'u', long = "mail")]
        usermail: String,
    },
    /// Remove a user's administrator rights
    DowngradeUser {
        /// Email address
        #[arg(short = 'u', long = "mail")]
        usermail: String,
    },
}

impl Admin {
    pub async fn run(self, pool: SqlitePool) -> Result<()> {
        match self {
            Self::CreateUser {
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
                            // Update the user as administrator
                            if let Err(e) =
                                sqlx::query("UPDATE Users SET is_admin = true WHERE id = ?")
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
                        Ok(())
                    }
                    Err(e) => {
                        bail!("Erreur lors de la création de l'utilisateur : {}", e);
                    }
                }
            }
            Self::ResetPassword { usermail, password } => {
                // Check whether the user exists
                match User::find_by_usermail(&usermail, &pool).await {
                    Ok(Some(_)) => {
                        // Hash the new password
                        let password_hash = match hash(password.as_bytes(), DEFAULT_COST) {
                            Ok(hash) => hash,
                            Err(e) => {
                                eprintln!("Erreur lors du hashage du mot de passe : {}", e);
                                process::exit(1);
                            }
                        };

                        // Update the password
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
                        Ok(())
                    }
                    Ok(None) => {
                        eprintln!("Utilisateur non trouvé : {}", usermail);
                        process::exit(1);
                    }
                    Err(e) => {
                        bail!("Erreur lors de la recherche de l'utilisateur : {}", e);
                    }
                }
            }
            Self::UpgradeUser { usermail } => {
                match User::find_by_usermail(&usermail, &pool).await {
                    Ok(Some(user)) => {
                        // Update the password
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
                        Ok(())
                    }
                    Ok(None) => {
                        bail!("Utilisateur non trouvé : {}", usermail);
                    }
                    Err(e) => {
                        eprintln!("Erreur lors de la recherche de l'utilisateur : {}", e);
                        process::exit(1);
                    }
                }
            }
            Self::DowngradeUser { usermail } => {
                match User::find_by_usermail(&usermail, &pool).await {
                    Ok(Some(user)) => {
                        // Update the password
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
                        Ok(())
                    }
                    Ok(None) => {
                        bail!("Utilisateur non trouvé : {}", usermail);
                    }
                    Err(e) => {
                        bail!("Erreur lors de la recherche de l'utilisateur : {}", e);
                    }
                }
            }
        }
    }
}
