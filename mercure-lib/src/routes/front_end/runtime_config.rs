use rocket::{State, get};
use sqlx::SqlitePool;
use serde::Serialize;

use crate::{
    auth::Authenticated,
    models::{AppState, AppStateConfigKey},
    routes::frontend::FrontendError,
    templates::{Template, context},
};

#[derive(Serialize)]
struct ConfigEntryForTemplate {
    key: String,
    value: String,
    value_type: String,  // "combo" or "text"
    combo_values: Vec<String>,
    description: String,
}

#[get("/runtime_config")]
pub async fn runtime_config_page_get(
    pool: &State<SqlitePool>,
    auth: Authenticated,
) -> Result<Template, FrontendError> {
    if !auth.user.is_admin {
        return Err(FrontendError::LogicalError("Admins only!".to_string()));
    }
    let config = AppState::get_all_typed(pool).await?;
    let entries: Vec<ConfigEntryForTemplate> = AppStateConfigKey::all()
        .iter()
        .map(|key| {
            let value = config.get(key).cloned().unwrap_or_default();
            let vt = key.value_type();
            ConfigEntryForTemplate {
                key: key.as_str().to_string(),
                value,
                value_type: if vt.is_combo() { "combo" } else { "text" }.to_string(),
                combo_values: vt.combo_values().to_vec(),
                description: key.description().to_string(),
            }
        })
        .collect();
    Ok(Template::render(
        "admin/runtime_config",
        context! {
            entries => entries,
        },
    ))
}
