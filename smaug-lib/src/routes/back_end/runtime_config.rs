use rocket::{State, get, post, serde::json::Json};
use sqlx::SqlitePool;

use crate::{
    auth::{AuthError, Authenticated},
    models::{AppState, AppStateConfigKey, appstate::ValueInput},
    routes::ApiResponse,
};

#[derive(Debug, serde::Deserialize)]
pub struct ConfigSetRequest {
    pub key: AppStateConfigKey,
    pub value: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ConfigEntry {
    pub key: AppStateConfigKey,
    pub value: String,
    pub value_input: ValueInput,
    pub description: &'static str,
}

#[get("/config/get")]
pub async fn runtime_config_get(
    pool: &State<SqlitePool>,
    auth: Authenticated,
) -> Result<Json<ApiResponse<Vec<ConfigEntry>>>, AuthError> {
    if !auth.user.is_admin {
        return Err(AuthError::AdminsOnly);
    }

    let mut entries = Vec::with_capacity(AppStateConfigKey::all().len());
    for key in AppStateConfigKey::all() {
        let value = AppState::get_str(*key, pool).await?;
        entries.push(ConfigEntry {
            key: *key,
            value,
            value_input: key.value_input(),
            description: key.description(),
        });
    }
    Ok(Json(ApiResponse::success(entries)))
}

#[post("/config/set", data = "<request>")]
pub async fn runtime_config_set(
    pool: &State<SqlitePool>,
    auth: Authenticated,
    request: Json<ConfigSetRequest>,
) -> Result<Json<ApiResponse<ConfigEntry>>, AuthError> {
    if !auth.user.is_admin {
        return Err(AuthError::AdminsOnly);
    }
    let key = request.key;
    let value = &request.value;
    // `parse_value` rejects unknown values with `ModelError::InvalidInput`, which
    // the edge maps to 400 instead of a generic 500.
    AppState::set_by_key(key, value, pool).await?;
    Ok(Json(ApiResponse::success(ConfigEntry {
        key,
        value: value.clone(),
        value_input: key.value_input(),
        description: key.description(),
    })))
}
