use hmac::{Hmac, Mac};
use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};
use rocket::serde::json::Json;
use rocket::{State, get, post};
use sha2::Sha256;
use sqlx::SqlitePool;

use crate::auth::{AuthError, Authenticated};
use crate::config::SmaugConfig;
use crate::models::form;
use crate::routes::ApiResponse;

type HmacSha256 = Hmac<Sha256>;

pub struct GiteaSignature<'r>(&'r str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for GiteaSignature<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        match request.headers().get_one("X-Gitea-Signature") {
            Some(sig) => Outcome::Success(GiteaSignature(sig)),
            None => Outcome::Error((Status::BadRequest, ())),
        }
    }
}

/// Helper to update cache and return correct HTTP status
///
/// # Arguments
///
/// - `pool` (`&State<SqlitePool>`) - sqlite database connection
/// - `config` (`&State<SmaugConfig>`) - configuration
///
/// # Returns
///
/// - `ApiResponse<String>` - The number of forms in the refreshed cache, or the error
async fn update_cache(
    pool: &State<SqlitePool>,
    config: &State<SmaugConfig>,
) -> Result<Json<ApiResponse<String>>, AuthError> {
    let v = form::CachedForm::refresh_cache(pool, config).await?;
    Ok(Json(ApiResponse::success(format!(
        "Le cache a été mis à jour avec succès ({} formulaires)",
        v.len()
    ))))
}

#[get("/update/cache/forms")]
pub async fn update_cache_get(
    auth: Authenticated,
    pool: &State<SqlitePool>,
    config: &State<SmaugConfig>,
) -> Result<Json<ApiResponse<String>>, AuthError> {
    if !auth.user.is_admin {
        return Err(AuthError::AdminsOnly);
    }
    update_cache(pool, config).await
}

#[post("/update/cache/forms/gitea", data = "<body>")]
pub async fn gitea_webhook_update_form_cache(
    signature: GiteaSignature<'_>,
    config: &State<SmaugConfig>,
    pool: &State<SqlitePool>,
    body: Vec<u8>,
) -> Result<Json<ApiResponse<String>>, AuthError> {
    let webhook_secret: &str = config.webhook_secret.as_str();

    // Decode the incoming hex signature into raw bytes. A malformed header is a
    // client error: answer 400 (InvalidSignature) so Gitea does not retry it.
    let sig_bytes = hex::decode(signature.0).map_err(|_| AuthError::InvalidSignature)?;

    // Initialize the HMAC hasher with your secret
    let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("Cannot create HMAC hasher from secret: {e}"))?;

    // Hash the raw request body
    mac.update(&body);

    // Verify the signature using constant-time equality
    mac.verify_slice(&sig_bytes)
        .map_err(|_| AuthError::InvalidCredentials)?;

    update_cache(pool, config).await
}
