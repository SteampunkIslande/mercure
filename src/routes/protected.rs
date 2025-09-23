use rocket::get;
use rocket::serde::json::Json;

use super::ApiResponse;
use crate::auth::Authenticated;

#[get("/me")]
pub async fn me(auth: Authenticated) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success(format!(
        "Utilisateur connecté: {}",
        auth.user.username
    )))
}

#[get("/protected")]
pub async fn protected_route(auth: Authenticated) -> Json<ApiResponse<String>> {
    Json(ApiResponse::success(format!(
        "Route protégée accessible pour l'utilisateur: {}",
        auth.user.username
    )))
}

// Route exemple qui ne nécessite pas d'authentification
#[get("/public")]
pub fn public_route() -> Json<ApiResponse<&'static str>> {
    Json(ApiResponse::success("Cette route est publique"))
}
