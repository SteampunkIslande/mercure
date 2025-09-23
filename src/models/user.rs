use bcrypt::{DEFAULT_COST, hash, verify};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use time::OffsetDateTime;

use crate::auth::AuthError;

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    password_hash: String,
    pub created_at: OffsetDateTime,
    pub last_login: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct NewUser {
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
            INSERT INTO users (username, password_hash, created_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&new_user.username)
        .bind(&password_hash)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        // Récupérer l'utilisateur créé
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, password_hash, created_at, last_login
            FROM users WHERE username = ?
            "#,
        )
        .bind(&new_user.username)
        .fetch_one(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    pub async fn find_by_id(id: i64, pool: &SqlitePool) -> Result<Option<User>, AuthError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, password_hash, created_at, last_login
            FROM users WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    pub async fn find_by_username(
        username: &str,
        pool: &SqlitePool,
    ) -> Result<Option<User>, AuthError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, password_hash, created_at, last_login
            FROM users WHERE username = ?
            "#,
        )
        .bind(username)
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
            UPDATE users SET last_login = ? WHERE id = ?
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
