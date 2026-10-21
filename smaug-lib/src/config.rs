use rocket::config;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "api")]
pub enum GitWebConfig {
    #[serde(rename = "gitea_v1")]
    GiteaV1 {
        base_url: String,
        owner: String,
        repo: String,
    },
    #[serde(rename = "gitlab_v4")]
    GitlabV4 {
        base_url: String,
        id: String, // ou u64 selon ce que vous préférez manipuler
    },
}

/// Additional configuration for Smaug
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct SmaugConfig {
    pub smaug_db: String,
    pub upload_dir: String,
    pub logs_dir: String,
    pub scripts_dir: String,
    pub analysis_dir: String,
    pub pipelines: GitWebConfig,
    pub webhook_secret: String,
}

impl Default for GitWebConfig {
    fn default() -> Self {
        GitWebConfig::GiteaV1 {
            base_url: "localhost:3333".into(),
            owner: "bioinfo".into(),
            repo: "smaug".into(),
        }
    }
}

impl Default for SmaugConfig {
    fn default() -> Self {
        Self {
            smaug_db: "sqlite://smaug.db".into(),
            upload_dir: "/home/charles/smaug/uploads".into(),
            logs_dir: "/home/charles/smaug/LOGS".into(),
            scripts_dir: "/home/charles/smaug/scripts".into(),
            analysis_dir: "/home/charles/smaug/analysis".into(),
            pipelines: GitWebConfig::default(),
            webhook_secret: String::new(),
        }
    }
}

pub fn get_smaug_config() -> SmaugConfig {
    config::Config::figment()
        .extract::<SmaugConfig>()
        .expect("Invalid rocket configuration")
}
