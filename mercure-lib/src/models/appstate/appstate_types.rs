//! Public type system for AppState configuration.
//!
//! This module exposes the strongly-typed configuration items, the registry enum,
//! and the validation/rendering utilities. Macro internals are private to this module.

use crate::models::ModelError;
use crate::models::appstate::appstate_macros::{ConfigItem, ValueInput};
use rocket::serde::{Deserialize, Serialize};

/// Helper macro to generate specific types (Enums or Date wrappers)
macro_rules! config_item_generator {
    // 1. Enum generation branch
    (
        $vis:vis enum $name:ident {
            db_key: $db_key:expr,
            description: $desc:expr,
            default: $default:ident,
            variants: [ $($variant:ident),* $(,)? ]$(,)?
        }
    ) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        $vis enum $name { $($variant),* }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let s = match self { $( Self::$variant => stringify!($variant), )* };
                f.write_str(s)
            }
        }

        impl std::str::FromStr for $name {
            type Err = ModelError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $( stringify!($variant) => Ok(Self::$variant), )*
                    _ => Err(ModelError::InvalidInput(format!(
                        "Invalid {} '{}'. Allowed: {}",
                        stringify!($name), s, [$(stringify!($variant)),*].join(", ")
                    ))),
                }
            }
        }

        impl ConfigItem for $name {
            const DB_KEY: &'static str = $db_key;
            const DESCRIPTION: &'static str = $desc;
            const DEFAULT_STR: &'static str = stringify!($default);

            fn default_value() -> Self { Self::$default }
            fn value_input() -> ValueInput {
                ValueInput::Combo(&[ $( stringify!($variant) ),* ])
            }
        }
    };

    // 2. Date generation branch
    (
        $vis:vis date $name:ident {
            db_key: $db_key:expr,
            description: $desc:expr,
            default: $default:expr$(,)?
        }
    ) => {
        /// A strongly typed newtype wrapper for a Date string (YYYY-MM-DD).
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        $vis struct $name(pub String);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = ModelError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.to_string()))
            }
        }

        impl ConfigItem for $name {
            const DB_KEY: &'static str = $db_key;
            const DESCRIPTION: &'static str = $desc;
            const DEFAULT_STR: &'static str = $default;

            fn default_value() -> Self { Self($default.to_string()) }
            fn value_input() -> ValueInput { ValueInput::Date }
        }
    };
}

/// The unified macro. It forwards item generation to `config_item_generator!`
/// and builds the `AppStateConfigKey` overarching enum automatically.
macro_rules! define_app_configs {
    (
        $($vis:vis $kind:ident $name:ident {
                $($body:tt)*
            }
        ),* $(,)?
    ) => {
        // Generate the underlying types (delegates to helper macro)
        $(
            config_item_generator! {
                $vis $kind $name { $($body)* }
            }
        )*

        // Generate the overarching registry for the Admin UI loop
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum AppStateConfigKey {
            $($name ),*
        }

        impl AppStateConfigKey {
            pub fn all() -> &'static [Self] { &[ $( Self::$name ),* ] }

            pub const fn as_str(&self) -> &'static str {
                match self { $( Self::$name => <$name as ConfigItem>::DB_KEY, )* }
            }

            pub const fn description(&self) -> &'static str {
                match self { $( Self::$name => <$name as ConfigItem>::DESCRIPTION, )* }
            }

            pub const fn default_value_str(&self) -> &'static str {
                match self { $( Self::$name => <$name as ConfigItem>::DEFAULT_STR, )* }
            }

            pub fn value_input(&self) -> ValueInput {
                match self { $( Self::$name => <$name as ConfigItem>::value_input(), )* }
            }

            /// Parses a raw string through the item's strongly-typed `FromStr`
            /// implementation, rejecting invalid values and returning the
            /// canonical string form that should be persisted.
            pub fn parse_value(&self, raw: &str) -> Result<String, ModelError> {
                match self {
                    $( Self::$name => Ok(<$name as std::str::FromStr>::from_str(raw)?.to_string()), )*
                }
            }
        }

        impl std::fmt::Display for AppStateConfigKey {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for AppStateConfigKey {
            type Err = ModelError;

            fn from_str(raw: &str) -> Result<Self, Self::Err> {
                Self::all()
                    .iter()
                    .copied()
                    .find(|key| key.as_str() == raw)
                    .ok_or_else(|| {
                        ModelError::InvalidInput(format!("Clé de config inconnue '{raw}'"))
                    })
            }
        }

        // A key is identified on the wire (HTML form, JSON) by its database key,
        // so a single canonical string round-trips through the whole stack.
        impl Serialize for AppStateConfigKey {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for AppStateConfigKey {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                raw.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

define_app_configs! {
    pub enum SigintBehavior {
        db_key: "on_sigint",
        description: "Actions à mener lorsque cette application reçoit SIGINT",
        default: StopJobs,
        variants: [StopJobs, WaitForJobCompletion],
    },

    pub enum ActiveRoutine {
        db_key: "active_routine",
        description: "Permet l'exécution immédiate de tous les runs...",
        default: RoutineOn,
        variants: [RoutineOff, RoutineOn],
    },

    // The new Date branch in action!
    pub date AppShutdownDate {
        db_key: "app_shutdown_date",
        description: "Date prévisionnelle d'arrêt de service de l'application",
        default: "2025-12-31",
    }
}

// The types are already public from the macro expansion (pub enum/struct)
// No need for pub use self::...
