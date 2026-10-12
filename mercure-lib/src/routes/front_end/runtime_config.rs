use rocket::{State, get};
use serde::Serialize;
use sqlx::SqlitePool;

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
    input_html: String,
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
            let vi = key.value_input();
            ConfigEntryForTemplate {
                key: key.as_str().to_string(),
                value: value.clone(),
                input_html: vi.render_html(key.as_str(), &value),
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
