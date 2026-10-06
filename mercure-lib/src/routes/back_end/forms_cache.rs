use hmac::{Hmac, Mac};
use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};
use rocket::{State, post};
use sha2::Sha256;

use crate::config::MercureConfig;

type HmacSha256 = Hmac<Sha256>;

// 1. Create a Request Guard to cleanly extract the Gitea signature header
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

// 2. The route takes both the signature header and the raw body bytes
#[post("/gitea-webhook", data = "<body>")]
pub fn gitea_webhook(
    signature: GiteaSignature<'_>,
    config: &State<MercureConfig>,
    body: Vec<u8>,
) -> Status {
    let webhook_secret: &str = config.webhook_secret.as_str();

    // Decode the incoming hex signature into raw bytes
    let Ok(sig_bytes) = hex::decode(signature.0) else {
        println!("Invalid hex formatting in signature header.");
        return Status::BadRequest;
    };

    // Initialize the HMAC hasher with your secret
    let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
        .expect("HMAC can take key of any size");

    // Hash the raw request body
    mac.update(&body);

    // Verify the signature using constant-time equality
    if mac.verify_slice(&sig_bytes).is_ok() {
        println!("Secure push registered! Verified webhook from Gitea.");
        Status::Ok
    } else {
        println!("Unauthorized webhook attempt: Signatures did not match.");
        Status::Unauthorized
    }
}
