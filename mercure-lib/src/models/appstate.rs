use std::collections::HashMap;

use rocket::serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::models::ModelError;

/// Macro to define AppState configuration keys with minimal boilerplate.
/// Usage:
/// ```ignore
/// define_appstate_config! {
///     AppstateLocked {
///         db_key: "appstate_locked",
///         description: "Lock AppState to prevent reset on next startup",
///         input: ValueInput::combo(&["true", "false"]),
///     },
///     ActiveRoutine {
///         db_key: "active_routine",
///         description: "Allow pending runs to transition to Running automatically",
///         input: ValueInput::combo(&["true", "false"]),
///     },
/// }
/// ```
#[macro_export]
macro_rules! define_appstate_config {
    (
        $(
            $variant:ident {
                db_key: $db_key:expr,
                description: $description:expr,
                input: $input:expr,
            }
        ),* $(,)?
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum AppStateConfigKey {
            $($variant,)*
        }

        impl AppStateConfigKey {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(AppStateConfigKey::$variant => $db_key,)*
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $($db_key => Some(AppStateConfigKey::$variant),)*
                    _ => None,
                }
            }

            pub fn all() -> &'static [AppStateConfigKey] {
                &[$(AppStateConfigKey::$variant,)*]
            }

            pub fn value_input(&self) -> ValueInput {
                match self {
                    $(AppStateConfigKey::$variant => $input,)*
                }
            }

            pub fn description(&self) -> &'static str {
                match self {
                    $(AppStateConfigKey::$variant => $description,)*
                }
            }
        }
    };
}

// Define the configuration using the macro
define_appstate_config! {
    AppstateLocked {
        db_key: "appstate_locked",
        description: "Verrouiller l'appstate pour éviter que l'état ne soit réinitialisé au prochain redémarrage (n'empêche pas les modifications).",
        input: ValueInput::combo(&["true", "false"]),
    },
    ActiveRoutine {
        db_key: "active_routine",
        description: "Permet l'exécution immédiate de tous les runs en attente. Passer cette valeur à false permet de suspendre les exécutions, par exemple pour drainer les jobs en cours avant un arrêt de l'application.",
        input: ValueInput::combo(&["true", "false"]),
    },
    SigintBehavior{
        db_key: "on_sigint",
        description: "Actions à mener lorsque cette application reçoit SIGINT",
        input: ValueInput::combo(&["StopJobs","WaitForJobCompletion"]),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "values")]
pub enum ValueInput {
    TextInput,
    Combo(Vec<String>),
}

impl ValueInput {
    pub fn text() -> Self {
        ValueInput::TextInput
    }

    pub fn combo(values: &[&str]) -> Self {
        ValueInput::Combo(values.iter().map(|s| s.to_string()).collect())
    }

    pub fn validate(&self, value: &str) -> Result<(), ModelError> {
        match self {
            ValueInput::Combo(allowed) => {
                if !allowed.iter().any(|v| v.eq_ignore_ascii_case(value)) {
                    return Err(ModelError::InvalidInput(format!(
                        "Invalid value '{}'. Allowed values: {:?}",
                        value, allowed
                    )));
                }
                Ok(())
            }
            ValueInput::TextInput => Ok(()),
        }
    }

    pub fn render_html(&self, name: &str, current_value: &str) -> String {
        match self {
            ValueInput::Combo(allowed) => {
                let mut html = format!(r#"<select class="config-select" name="{}">"#, name);
                for opt in allowed {
                    let selected = if opt.eq_ignore_ascii_case(current_value) {
                        " selected"
                    } else {
                        ""
                    };
                    html.push_str(&format!(
                        r#"<option value="{}"{selected}>{}</option>"#,
                        opt, opt
                    ));
                }
                html.push_str("</select>");
                html
            }
            ValueInput::TextInput => {
                format!(
                    r#"<input type="text" class="config-input" name="{}" value="{}">"#,
                    name,
                    html_escape::encode_text(current_value)
                )
            }
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
        key.value_input().validate(value)?;
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
