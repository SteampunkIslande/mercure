//! AppState module - runtime configuration management.
//!
//! Public API:
//! - `AppState` - Main entry point for getting/setting config values
//! - `AppStateConfigKey` - Registry enum for all config keys
//! - `ConfigItem` - Trait for strongly-typed config values
//! - `ValueInput` - Enum for HTML input rendering (Combo, Date)
//! - `SigintBehavior`, `ActiveRoutine`, `AppShutdownDate` - Concrete config types

mod appstate_impl;
mod appstate_macros;
mod appstate_types;

pub use appstate_impl::AppState;
pub use appstate_macros::{ConfigItem, ValueInput};
pub use appstate_types::{ActiveRoutine, AppShutdownDate, AppStateConfigKey, SigintBehavior};
