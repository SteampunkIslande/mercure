use serde::Deserialize;
use std::path::PathBuf;

/// Additional configuration for Mercure
#[derive(Deserialize)]
pub struct MercureConfig {
    pub mercure_db: PathBuf,
}
