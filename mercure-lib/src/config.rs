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

/// Additional configuration for Mercure
#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct MercureConfig {
    pub mercure_db: String,
    pub upload_dir: String,
    pub logs_dir: String,
    pub pipelines: GitWebConfig,
}

impl Default for GitWebConfig {
    fn default() -> Self {
        GitWebConfig::GiteaV1 {
            base_url: "localhost:3333".into(),
            owner: "bioinfo".into(),
            repo: "mercure".into(),
        }
    }
}

impl Default for MercureConfig {
    fn default() -> Self {
        Self {
            mercure_db: "sqlite://mercure.db".into(),
            upload_dir: "/home/charles/mercure/uploads".into(),
            logs_dir: "/home/charles/mercure/LOGS".into(),
            pipelines: GitWebConfig::default(),
        }
    }
}

pub fn get_mercure_config() -> MercureConfig {
    config::Config::figment()
        .extract::<MercureConfig>()
        .expect("Invalid rocket configuration")
}
