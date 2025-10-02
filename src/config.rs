use serde::Deserialize;
use std::path::PathBuf;

/// Additional configuration for Mercure
#[derive(Deserialize)]
pub struct MercureConfig {
    pub mercure_db: PathBuf,
    pub sequencers_folder: PathBuf,
    pub upload_folder: PathBuf,
}

impl Default for MercureConfig {
    fn default() -> Self {
        Self {
            mercure_db: "sqlite://mercure.db".into(),
            sequencers_folder: "/data/raw/sequenceurs".into(),
            upload_folder: "/home/charles/mercure/uploads".into(),
        }
    }
}
