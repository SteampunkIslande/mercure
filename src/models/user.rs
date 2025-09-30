use bcrypt::{DEFAULT_COST, hash, verify};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

use crate::auth::AuthError;

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub usermail: String,
    pub username: String,
    #[serde(skip_serializing)]
    password_hash: String,
    pub created_at: OffsetDateTime,
    pub last_login: Option<OffsetDateTime>,
    pub is_admin: bool,
}

#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub usermail: String,
    pub username: String,
    pub password: String,
}

impl User {
    pub async fn create(new_user: NewUser, pool: &SqlitePool) -> Result<User, AuthError> {
        let password_hash = hash(new_user.password.as_bytes(), DEFAULT_COST)
            .map_err(|_| AuthError::DatabaseError("Erreur de hachage du mot de passe".into()))?;

        let now = OffsetDateTime::now_utc();

        // Insérer l'utilisateur
        sqlx::query(
            r#"
            INSERT INTO Users (usermail, username, password_hash, created_at, is_admin)
            VALUES (?, ?, ?, ?, false)
            "#,
        )
        .bind(&new_user.usermail)
        .bind(&new_user.username)
        .bind(&password_hash)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        // Récupérer l'utilisateur créé
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, usermail, username, password_hash, created_at, last_login, is_admin
            FROM Users WHERE usermail = ?
            "#,
        )
        .bind(&new_user.usermail)
        .fetch_one(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    pub async fn list_users(pool: &SqlitePool) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(r#"SELECT * FROM Users"#)
            .fetch_all(pool)
            .await
    }

    pub async fn find_by_id(id: i64, pool: &SqlitePool) -> Result<Option<User>, AuthError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, usermail, username, password_hash, created_at, last_login, is_admin
            FROM Users WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    pub async fn find_by_usermail(
        usermail: &str,
        pool: &SqlitePool,
    ) -> Result<Option<User>, AuthError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, usermail, username, password_hash, created_at, last_login, is_admin
            FROM Users WHERE usermail = ?
            "#,
        )
        .bind(usermail)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    pub async fn verify_password(&self, password: &str) -> bool {
        verify(password.as_bytes(), &self.password_hash).unwrap_or(false)
    }

    pub async fn update_last_login(&mut self, pool: &SqlitePool) -> Result<(), AuthError> {
        let now = OffsetDateTime::now_utc();
        self.last_login = Some(now);

        sqlx::query(
            r#"
            UPDATE Users SET last_login = ? WHERE id = ?
            "#,
        )
        .bind(now)
        .bind(self.id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
