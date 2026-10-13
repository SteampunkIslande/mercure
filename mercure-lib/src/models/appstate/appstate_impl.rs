use crate::models::ModelError;
use crate::models::appstate::{AppStateConfigKey, ConfigItem};
use sqlx::SqlitePool;

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

    /// Reads and parses the strongly typed value for `T`, falling back to its
    /// declared default when the key is missing or holds an invalid value.
    pub async fn get<T: ConfigItem>(pool: &SqlitePool) -> Result<T, ModelError> {
        let raw: Option<String> = sqlx::query_scalar("SELECT value FROM AppState WHERE key = ?")
            .bind(T::DB_KEY)
            .fetch_optional(pool)
            .await?;

        Ok(raw
            .and_then(|s| s.parse::<T>().ok())
            .unwrap_or_else(T::default_value))
    }

    /// Reads the raw string value for a key, falling back to its declared
    /// default. Intended for the admin UI, which renders untyped form inputs.
    pub async fn get_str(key: AppStateConfigKey, pool: &SqlitePool) -> Result<String, ModelError> {
        let raw: Option<String> = sqlx::query_scalar("SELECT value FROM AppState WHERE key = ?")
            .bind(key.as_str())
            .fetch_optional(pool)
            .await?;

        Ok(raw.unwrap_or_else(|| key.default_value_str().to_string()))
    }

    /// Persists a strongly typed value.
    /// Usage: `AppState::set(SigintBehavior::StopJobs, &pool).await?`
    /// Usage: `AppState::set(AppShutdownDate("2024-05-12".into()), &pool).await?`
    pub async fn set<T: ConfigItem>(value: T, pool: &SqlitePool) -> Result<(), ModelError> {
        Self::write(T::DB_KEY, &value.to_string(), pool).await
    }

    /// Persists a value received as an untyped string (e.g. from the admin API),
    /// validating and canonicalizing it through the strongly typed item first.
    pub async fn set_by_key(
        key: AppStateConfigKey,
        value: &str,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        let canonical = key.parse_value(value)?;
        Self::write(key.as_str(), &canonical, pool).await
    }

    async fn write(key: &str, value: &str, pool: &SqlitePool) -> Result<(), ModelError> {
        sqlx::query("INSERT OR REPLACE INTO AppState (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(pool)
            .await?;
        Ok(())
    }
}
