use hmac::{Hmac, Mac};
use log::error;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};
use rocket::{State, get, post};
use sha2::Sha256;
use sqlx::SqlitePool;

use crate::auth::Authenticated;
use crate::config::MercureConfig;
use crate::models::form;

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
/// - `config` (`&State<MercureConfig>`) - configuration
///
/// # Returns
///
/// - `(Status, String)` - A tuple of HTTP Status with a custom message
async fn update_cache(pool: &State<SqlitePool>, config: &State<MercureConfig>) -> (Status, String) {
    match form::CachedForm::refresh_cache(pool, config).await {
        Ok(v) => (
            Status::Ok,
            format!(
                "Le cache a été mis à jour avec succès ({} formulaires)",
                v.len()
            ),
        ),
        Err(e) => {
            error!("{e}");
            (Status::InternalServerError, e.to_string())
        }
    }
}

#[get("/update/cache/forms")]
pub async fn update_cache_get(
    _auth: Authenticated,
    pool: &State<SqlitePool>,
    config: &State<MercureConfig>,
) -> (Status, String) {
    update_cache(pool, config).await
}

#[post("/update/cache/forms/gitea", data = "<body>")]
pub async fn gitea_webhook_update_form_cache(
    signature: GiteaSignature<'_>,
    config: &State<MercureConfig>,
    pool: &State<SqlitePool>,
    body: Vec<u8>,
) -> (Status, String) {
    let webhook_secret: &str = config.webhook_secret.as_str();

    // Decode the incoming hex signature into raw bytes
    let Ok(sig_bytes) = hex::decode(signature.0) else {
        return (
            Status::BadRequest,
            "Invalid hex formatting in signature header.".into(),
        );
    };

    // Initialize the HMAC hasher with your secret
    let Ok(mut mac) = HmacSha256::new_from_slice(webhook_secret.as_bytes()) else {
        return (
            Status::InternalServerError,
            "Cannot create HMAC hasher from secret".into(),
        );
    };

    // Hash the raw request body
    mac.update(&body);

    // Verify the signature using constant-time equality
    if mac.verify_slice(&sig_bytes).is_ok() {
        update_cache(pool, config).await
    } else {
        (Status::Unauthorized, "Nope, Unauthorized".into())
    }
}
