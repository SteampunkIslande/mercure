use crate::models::ModelError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryInfo {
    pub name: String,
    pub path: String,
}

/// Fonction générique pour lister les dossiers dans un répertoire donné
pub fn list_directory(base_path: &str) -> Result<Vec<DirectoryInfo>, ModelError> {
    if !Path::new(base_path).exists() {
        return Ok(Vec::new()); // Retourne une liste vide si le chemin n'existe pas
    }

    let entries = fs::read_dir(base_path).map_err(|e| {
        ModelError::FormError(format!(
            "Erreur lors de la lecture du répertoire {}: {}",
            base_path, e
        ))
    })?;

    let mut directories = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| {
            ModelError::FormError(format!("Erreur lors de la lecture d'une entrée: {}", e))
        })?;

        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    directories.push(DirectoryInfo {
                        name: name_str.to_string(),
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }

    // Trier par nom pour une présentation cohérente
    directories.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(directories)
}
