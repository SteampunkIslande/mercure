use rocket::{State, get, post, serde::json::Json};
use sqlx::SqlitePool;

use crate::{
    auth::Authenticated,
    models::{AppState, AppStateConfigKey, appstate::ValueType},
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
    pub value_type: ValueType,
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
    match AppState::get_all_typed(pool).await {
        Ok(conf) => {
            let entries: Vec<ConfigEntry> = AppStateConfigKey::all()
                .iter()
                .map(|key| ConfigEntry {
                    key: *key,
                    value: conf.get(key).cloned().unwrap_or_default(),
                    value_type: key.value_type(),
                    description: key.description(),
                })
                .collect();
            Json(ApiResponse::success(entries))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
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
    match AppState::set(key, value, pool).await {
        Ok(_) => Json(ApiResponse::success(ConfigEntry {
            key,
            value: value.clone(),
            value_type: key.value_type(),
            description: key.description(),
        })),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}
