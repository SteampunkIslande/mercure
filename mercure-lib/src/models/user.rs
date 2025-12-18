use bcrypt::{DEFAULT_COST, hash, verify};
use serde::{Deserialize, Serialize};
use sqlx::Row;
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

impl Default for User {
    fn default() -> Self {
        Self {
            id: Default::default(),
            usermail: Default::default(),
            username: Default::default(),
            password_hash: Default::default(),
            created_at: OffsetDateTime::now_utc(),
            last_login: Default::default(),
            is_admin: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub usermail: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordUpdate<'a> {
    pub user_id: i64,
    pub old_password: Option<&'a str>,
    pub new_password: &'a str,
}

impl User {
    pub async fn create(new_user: NewUser, pool: &SqlitePool) -> Result<User, AuthError> {
        let password_hash = hash(new_user.password.as_bytes(), DEFAULT_COST)?;

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

    async fn actually_update_password(
        pool: &SqlitePool,
        new_password_hash: &str,
        user_id: i64,
    ) -> Result<(), AuthError> {
        sqlx::query(
            r#"
            UPDATE Users
            SET password_hash = ?
            WHERE id = ?
            "#,
        )
        .bind(new_password_hash)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn update_password(
        password_update: PasswordUpdate<'_>,
        pool: &SqlitePool,
    ) -> Result<(), AuthError> {
        // Administrator is updating password
        if password_update.old_password.is_none() {
            // No need to check, this comes from an administrator
            let new_password_hash = hash(password_update.new_password.as_bytes(), DEFAULT_COST)?;
            return Self::actually_update_password(
                pool,
                &new_password_hash,
                password_update.user_id,
            )
            .await;
        }
        let correct_old_password_hash: String =
            sqlx::query("SELECT password_hash FROM Users WHERE id = ?")
                .bind(password_update.user_id)
                .fetch_one(pool)
                .await?
                .try_get("password_hash")?;
        let new_password_hash = hash(password_update.new_password.as_bytes(), DEFAULT_COST)?;
        if !verify(
            password_update
                .old_password
                .ok_or(AuthError::NoPassword)?
                .as_bytes(),
            &correct_old_password_hash,
        )? {
            return Err(AuthError::InvalidCredentials);
        }
        Self::actually_update_password(pool, &new_password_hash, password_update.user_id).await
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
