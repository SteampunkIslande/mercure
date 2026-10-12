use std::collections::HashMap;

use rocket::serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::models::ModelError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppStateConfigKey {
    AppstateLocked,
    ActiveRoutine,
}

impl AppStateConfigKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppStateConfigKey::AppstateLocked => "appstate_locked",
            AppStateConfigKey::ActiveRoutine => "active_routine",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "appstate_locked" => Some(AppStateConfigKey::AppstateLocked),
            "active_routine" => Some(AppStateConfigKey::ActiveRoutine),
            _ => None,
        }
    }

    pub fn all() -> &'static [AppStateConfigKey] {
        &[
            AppStateConfigKey::AppstateLocked,
            AppStateConfigKey::ActiveRoutine,
        ]
    }

    pub fn value_type(&self) -> ValueType {
        match self {
            AppStateConfigKey::AppstateLocked => {
                ValueType::Combo(vec!["true".to_string(), "false".to_string()])
            }
            AppStateConfigKey::ActiveRoutine => {
                ValueType::Combo(vec!["true".to_string(), "false".to_string()])
            }
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AppStateConfigKey::AppstateLocked => "Lock AppState to prevent reset on next startup",
            AppStateConfigKey::ActiveRoutine => {
                "Allow pending runs to transition to Running automatically"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "values")]
pub enum ValueType {
    TextInput,
    Combo(Vec<String>),
}

impl ValueType {
    pub fn is_combo(&self) -> bool {
        matches!(self, ValueType::Combo(_))
    }

    pub fn combo_values(&self) -> &[String] {
        match self {
            ValueType::Combo(v) => v,
            ValueType::TextInput => &[],
        }
    }
}

pub struct AppState {}

impl AppState {
    pub async fn on_startup(pool: &SqlitePool) -> Result<(), ModelError> {
        if Self::appstate_is_locked(pool).await? {
            Ok(())
        } else {
            Ok(Self::set(AppStateConfigKey::ActiveRoutine, "true", pool).await?)
        }
    }

    pub async fn get(
        key: AppStateConfigKey,
        pool: &SqlitePool,
    ) -> Result<Option<String>, ModelError> {
        Ok(
            sqlx::query_scalar(r#"SELECT value FROM AppState WHERE key = ?"#)
                .bind(key.as_str())
                .fetch_optional(pool)
                .await?,
        )
    }

    async fn get_all(pool: &SqlitePool) -> Result<HashMap<String, String>, ModelError> {
        Ok(sqlx::query(r#"SELECT key,value FROM AppState"#)
            .fetch_all(pool)
            .await?
            .into_iter()
            .filter_map(|row| Some((row.try_get("key").ok()?, row.try_get("value").ok()?)))
            .collect())
    }

    pub async fn get_all_typed(
        pool: &SqlitePool,
    ) -> Result<HashMap<AppStateConfigKey, String>, ModelError> {
        let all = Self::get_all(pool).await?;
        Ok(all
            .into_iter()
            .filter_map(|(k, v)| AppStateConfigKey::from_str(&k).map(|key| (key, v)))
            .collect())
    }

    pub async fn set(
        key: AppStateConfigKey,
        value: &str,
        pool: &SqlitePool,
    ) -> Result<(), ModelError> {
        Self::validate_value(key, value)?;
        sqlx::query(
            r#"
        INSERT OR REPLACE INTO AppState (key, value) VALUES (?, ?)
        "#,
        )
        .bind(key.as_str())
        .bind(value)
        .execute(pool)
        .await?;
        Ok(())
    }

    fn validate_value(key: AppStateConfigKey, value: &str) -> Result<(), ModelError> {
        match key.value_type() {
            ValueType::Combo(allowed) => {
                if !allowed.iter().any(|v| v.eq_ignore_ascii_case(value)) {
                    return Err(ModelError::InvalidInput(format!(
                        "Invalid value '{}' for key '{}'. Allowed values: {:?}",
                        value,
                        key.as_str(),
                        allowed
                    )));
                }
                Ok(())
            }
            ValueType::TextInput => Ok(()),
        }
    }

    pub async fn lock_appstate(pool: &SqlitePool) -> Result<(), ModelError> {
        Ok(Self::set(AppStateConfigKey::AppstateLocked, "true", pool).await?)
    }

    pub async fn unlock_appstate(pool: &SqlitePool) -> Result<(), ModelError> {
        Ok(Self::set(AppStateConfigKey::AppstateLocked, "false", pool).await?)
    }

    pub async fn appstate_is_locked(pool: &SqlitePool) -> Result<bool, ModelError> {
        Ok(Self::get(AppStateConfigKey::AppstateLocked, pool)
            .await?
            .map(|s| s.eq_ignore_ascii_case("true"))
            .unwrap_or(false))
    }

    pub async fn active_routine(pool: &SqlitePool) -> Result<bool, ModelError> {
        Ok(Self::get(AppStateConfigKey::ActiveRoutine, pool)
            .await?
            .map(|s| s.eq_ignore_ascii_case("true"))
            .unwrap_or(true))
    }
}
