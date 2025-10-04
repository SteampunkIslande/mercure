use rocket::config;
use serde::Deserialize;

/// Additional configuration for Mercure
#[derive(Deserialize)]
pub struct MercureConfig {
    pub mercure_db: String,
    pub sequencers_folder: String,
    pub upload_folder: String,
    pub static_dir: String,
}

impl Default for MercureConfig {
    fn default() -> Self {
        Self {
            mercure_db: "sqlite://mercure.db".into(),
            sequencers_folder: "/data/raw/sequenceurs".into(),
            upload_folder: "/home/charles/mercure/uploads".into(),
            static_dir: "/home/charles/mercure/static".into(),
        }
    }
}

pub fn get_mercure_config() -> MercureConfig {
    config::Config::figment()
        .extract::<MercureConfig>()
        .unwrap_or_default()
}
