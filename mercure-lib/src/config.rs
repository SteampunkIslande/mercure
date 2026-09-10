use rocket::config;
use serde::Deserialize;

/// Additional configuration for Mercure
#[derive(Deserialize)]
#[serde(default)]
pub struct MercureConfig {
    pub mercure_db: String,
    pub sequencers_dir: String,
    pub upload_dir: String,
    pub analysis_dir: String,
    pub static_dir: String,
    pub pipeline_dir: String,
    pub jobs_dir: String,
    pub logs_dir: String,
    pub ont_dir: String,
    pub check_run_completed: String,
    pub post_run_script: String,
}

impl Default for MercureConfig {
    fn default() -> Self {
        Self {
            mercure_db: "sqlite://mercure.db".into(),
            sequencers_dir: "/home/charles/mercure/sequenceurs".into(),
            upload_dir: "/home/charles/mercure/uploads".into(),
            analysis_dir: "/home/charles/mercure/analysis".into(),
            static_dir: "/home/charles/mercure/static".into(),
            pipeline_dir: "/home/charles/mercure/pipelines".into(),
            jobs_dir: "/home/charles/mercure/JOBS".into(),
            logs_dir: "/home/charles/mercure/LOGS".into(),
            ont_dir: "/home/charles/mercure/ont".into(),
            check_run_completed: "/home/charles/mercure/scripts/run-completed.sh".into(),
            post_run_script: "/home/charles/mercure/post-run.sh".into(),
        }
    }
}

pub fn get_mercure_config() -> MercureConfig {
    config::Config::figment()
        .extract::<MercureConfig>()
        .expect("Invalid rocket configuration")
}
