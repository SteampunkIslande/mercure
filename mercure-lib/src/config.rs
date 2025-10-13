use rocket::config;
use serde::Deserialize;

/// Additional configuration for Mercure
#[derive(Deserialize)]
pub struct MercureConfig {
    pub mercure_db: String,
    pub sequencers_folder: String,
    pub upload_folder: String,
    pub analysis_folder: String,
    pub static_dir: String,
    pub pipeline_dir: String,
    pub todo_dir: String,
}

impl Default for MercureConfig {
    fn default() -> Self {
        Self {
            mercure_db: "sqlite://mercure.db".into(),
            sequencers_folder: "/data/raw/sequenceurs".into(),
            upload_folder: "/home/charles/mercure/uploads".into(),
            analysis_folder: "/home/charles/mercure/analysis".into(),
            static_dir: "/home/charles/mercure/static".into(),
            pipeline_dir: "/home/charles/mercure/pipelines".into(),
            todo_dir: "/home/charles/mercure/jobs/todo".into(),
        }
    }
}

pub fn get_mercure_config() -> MercureConfig {
    config::Config::figment()
        .extract::<MercureConfig>()
        .unwrap_or_default()
}
