use rocket::{State, get, post, serde::json::Json};
use sqlx::SqlitePool;

use crate::{
    auth::Authenticated,
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
) -> Json<ApiResponse<Vec<ConfigEntry>>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error("Admins only!"));
    }

    let mut entries = Vec::with_capacity(AppStateConfigKey::all().len());
    for key in AppStateConfigKey::all() {
        match AppState::get_str(*key, pool).await {
            Ok(value) => entries.push(ConfigEntry {
                key: *key,
                value,
                value_input: key.value_input(),
                description: key.description(),
            }),
            Err(e) => return Json(ApiResponse::error(e.to_string())),
        }
    }
    Json(ApiResponse::success(entries))
}

#[post("/config/set", data = "<request>")]
pub async fn runtime_config_set(
    pool: &State<SqlitePool>,
    auth: Authenticated,
    request: Json<ConfigSetRequest>,
) -> Json<ApiResponse<ConfigEntry>> {
    if !auth.user.is_admin {
        return Json(ApiResponse::error("Admins only!"));
    }
    let key = request.key;
    let value = &request.value;
    match AppState::set_by_key(key, value, pool).await {
        Ok(_) => Json(ApiResponse::success(ConfigEntry {
            key,
            value: value.clone(),
            value_input: key.value_input(),
            description: key.description(),
        })),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}
