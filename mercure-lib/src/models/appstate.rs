use std::collections::HashMap;
use std::str::FromStr;

use rocket::serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::models::ModelError;

/// Trait for config types to provide metadata and HTML rendering.
pub trait ConfigType:
    Sized + FromStr<Err = ModelError> + Default + Clone + Send + Sync + 'static
{
    const DB_KEY: &'static str;
    const DESCRIPTION: &'static str;
    const DEFAULT: &'static str;

    fn value_input() -> ValueInput;
    fn render_html(name: &str, current_value: &str) -> String {
        Self::value_input().render_html(name, current_value)
    }
}

/// Trait for config values that can be parsed from strings.
pub trait ConfigValue: std::fmt::Display + Send + Sync {}
impl<T> ConfigValue for T where
    T: std::fmt::Display + Send + Sync + FromStr<Err = ModelError> + Default + Clone + 'static
{
}

/// Define an enum config type.
/// Usage:
/// ```
/// enum_config! {
///     SigintBehavior {
///         db_key: "on_sigint",
///         description: "Actions when receiving SIGINT",
///         variants: [StopJobs, WaitForJobCompletion],
///         default: StopJobs,
///     }
/// }
/// ```
#[macro_export]
macro_rules! enum_config {
    (
        $(
            $type_name:ident {
                db_key: $db_key:expr,
                description: $description:expr,
                variants: [$($variant:ident),* $(,)?],
                default: $default:ident,
            }
        ),* $(,)?
    ) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
            #[serde(rename_all = "PascalCase")]
            pub enum $type_name {
                $($variant,)*
            }

            impl $type_name {
                pub const fn all() -> &'static [$type_name] {
                    &[$($type_name::$variant,)*]
                }

                pub fn as_str(&self) -> &'static str {
                    match self {
                        $($type_name::$variant => stringify!($variant),)*
                    }
                }
            }

            impl FromStr for $type_name {
                type Err = ModelError;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    match s {
                        $(stringify!($variant) => Ok($type_name::$variant),)*
                        _ => Err(ModelError::InvalidInput(format!(
                            "Invalid {} '{}'. Allowed: {}",
                            stringify!($type_name),
                            s,
                            [$($type_name::$variant.as_str()),*].join(", ")
                        ))),
                    }
                }
            }

            impl Default for $type_name {
                fn default() -> Self {
                    $type_name::$default
                }
            }

            impl std::fmt::Display for $type_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.as_str())
                }
            }

            impl crate::models::appstate::ConfigType for $type_name {
                const DB_KEY: &'static str = $db_key;
                const DESCRIPTION: &'static str = $description;
                const DEFAULT: &'static str = stringify!($default);

                fn value_input() -> ValueInput {
                    ValueInput::combo(&[$(stringify!($variant)),*])
                }
            }
        )*
    };
}

/// Macro to define AppState configuration keys from config types.
/// Usage:
/// ```
/// define_appstate_config! {
///     SigintBehavior,
///     ActiveRoutine,
/// }
/// ```
#[macro_export]
macro_rules! define_appstate_config {
    ($($type_name:ident),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum AppStateConfigKey {
            $($type_name,)*
        }

        impl AppStateConfigKey {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(AppStateConfigKey::$type_name => <$type_name as crate::models::appstate::ConfigType>::DB_KEY,)*
                }
            }

            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    $(<$type_name as crate::models::appstate::ConfigType>::DB_KEY => Some(AppStateConfigKey::$type_name),)*
                    _ => None,
                }
            }

            pub fn all() -> &'static [AppStateConfigKey] {
                &[$(AppStateConfigKey::$type_name,)*]
            }

            pub fn value_input(&self) -> ValueInput {
                match self {
                    $(AppStateConfigKey::$type_name => <$type_name as crate::models::appstate::ConfigType>::value_input(),)*
                }
            }

            pub fn description(&self) -> &'static str {
                match self {
                    $(AppStateConfigKey::$type_name => <$type_name as crate::models::appstate::ConfigType>::DESCRIPTION,)*
                }
            }

            pub fn default_value(&self) -> &'static str {
                match self {
                    $(AppStateConfigKey::$type_name => <$type_name as crate::models::appstate::ConfigType>::DEFAULT,)*
                }
            }

            pub fn parse_value(&self, s: &str) -> Result<Box<dyn crate::models::appstate::ConfigValue>, ModelError> {
                match self {
                    $(AppStateConfigKey::$type_name => Ok(Box::new(s.parse::<$type_name>()?)),)*
                }
            }
        }
    };
}

// Define config types
enum_config! {
    SigintBehavior {
        db_key: "on_sigint",
        description: "Actions à mener lorsque cette application reçoit SIGINT",
        variants: [StopJobs, WaitForJobCompletion],
        default: StopJobs,
    }
}

enum_config! {
    ActiveRoutine {
        db_key: "active_routine",
        description: "Permet l'exécution immédiate de tous les runs en attente. Passer cette valeur à false permet de suspendre les exécutions, par exemple pour drainer les jobs en cours avant un arrêt de l'application.",
        variants: [RoutineOff, RoutineOn],
        default: RoutineOn,
    }
}

// Register config types in AppStateConfigKey
define_appstate_config! {
    SigintBehavior,
    ActiveRoutine,
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
                        "Invalid value '{}'. Allowed values: {}",
                        value,
                        allowed.join(", ")
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
        let existing = Self::get_all(pool).await?;
        for key in AppStateConfigKey::all() {
            let db_key = key.as_str();
            if !existing.contains_key(db_key) {
                Self::set(*key, key.default_value(), pool).await?;
            }
        }
        Ok(())
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

    pub async fn active_routine(pool: &SqlitePool) -> Result<bool, ModelError> {
        Ok(Self::get(AppStateConfigKey::ActiveRoutine, pool)
            .await?
            .map(|s| s.parse::<ActiveRoutine>())
            .transpose()?
            .map(|ar| matches!(ar, ActiveRoutine::RoutineOn))
            .unwrap_or(true))
    }

    pub async fn sigint_behavior(pool: &SqlitePool) -> Result<SigintBehavior, ModelError> {
        Ok(Self::get(AppStateConfigKey::SigintBehavior, pool)
            .await?
            .map(|s| s.parse())
            .transpose()?
            .unwrap_or_default())
    }
}
