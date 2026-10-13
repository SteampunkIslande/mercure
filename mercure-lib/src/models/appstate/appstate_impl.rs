use crate::models::ModelError;
use crate::models::appstate::{
    ActiveRoutine, AppShutdownDate, AppStateConfigKey, ConfigItem, SigintBehavior,
};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

pub struct AppState;

impl AppState {
    pub async fn on_startup(pool: &SqlitePool) -> Result<(), ModelError> {
        for key in AppStateConfigKey::all() {
            sqlx::query("INSERT OR IGNORE INTO AppState (key, value) VALUES (?, ?)")
                .bind(key.as_str())
                .bind(key.default_value_str())
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    /// Safely gets the strongly typed value
    pub async fn get<T: ConfigItem>(pool: &SqlitePool) -> Result<T, ModelError> {
        let val: Option<String> = sqlx::query_scalar("SELECT value FROM AppState WHERE key = ?")
            .bind(T::DB_KEY)
            .fetch_optional(pool)
            .await?;

        match val {
            Some(s) => Ok(s.parse::<T>().unwrap_or_else(|_| T::default_value())),
            None => Ok(T::default_value()),
        }
    }

    /// Fetches everything as strings for the Admin UI renderer
    pub async fn get_all(pool: &SqlitePool) -> Result<HashMap<String, String>, ModelError> {
        Ok(sqlx::query("SELECT key, value FROM AppState")
            .fetch_all(pool)
            .await?
            .into_iter()
            .filter_map(|row| Some((row.try_get("key").ok()?, row.try_get("value").ok()?)))
            .collect())
    }

    /// The new perfectly-typed setter.
    /// Usage: `AppState::set(SigintBehavior::StopJobs, &pool).await?`
    /// Usage: `AppState::set(AppShutdownDate("2024-05-12".into()), &pool).await?`
    pub async fn set<T: ConfigItem>(value: T, pool: &SqlitePool) -> Result<(), ModelError> {
        sqlx::query("INSERT OR REPLACE INTO AppState (key, value) VALUES (?, ?)")
            .bind(T::DB_KEY)
            .bind(value.to_string()) // Handled gracefully by the Display implementation
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Sets a config value by key, parsing the string to the appropriate type.
    /// Used by the Admin API which receives untyped strings.
    pub async fn set_by_key(
        key: AppStateConfigKey,
        value: &str,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        match key {
            AppStateConfigKey::SigintBehavior => {
                let parsed: SigintBehavior = value.parse()?;
                Self::set(parsed, pool).await
            }
            AppStateConfigKey::ActiveRoutine => {
                let parsed: ActiveRoutine = value.parse()?;
                Self::set(parsed, pool).await
            }
            AppStateConfigKey::AppShutdownDate => {
                let parsed: AppShutdownDate = value.parse()?;
                Self::set(parsed, pool).await
            }
        }
    }
}
