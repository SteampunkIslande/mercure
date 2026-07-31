/// Ce module contient des fonctions pour vérifier les révisions git des fichiers de lanceur.
use crate::config;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TemplateCheckError {
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error("Le fichier template n'est pas suivi par git.")]
    UntrackedFile,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    StripPrefixError(#[from] std::path::StripPrefixError),
}

pub async fn exists_launcher(pipeline_name: &str, launcher_name: &str) -> bool {
    let config = config::get_mercure_config();
    let launcher_path = Path::new(&config.pipeline_dir)
        .join(pipeline_name)
        .join("launchers")
        .join(launcher_name);
    launcher_path.exists()
}
