//! AppState module - runtime configuration management.
//!
//! Public API:
//! - `AppState` - Main entry point for getting/setting config values
//! - `ConfigItem` - Trait for strongly-typed config values
//! - `ValueInput` - Enum for HTML input rendering (Combo, Date)
//! - `AppStateConfigKey` - Registry enum for all config keys
//! - `SigintBehavior`, `ActiveRoutine`, `AppShutdownDate` - Concrete config types

pub mod appstate_defs;
mod appstate_impl;
pub mod appstate_macros;
pub mod appstate_types;

// Re-export the core types from the macro module
pub use appstate_defs::*;
pub use appstate_impl::AppState;
pub use appstate_macros::{ConfigItem, ValueInput};
