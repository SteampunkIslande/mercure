use crate::models::ModelError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryInfo {
    pub name: String,
    pub path: String,
}

/// Utilitaires pour gérer les différents types de dossiers d'entrée
pub struct DirectoryUtils;

impl DirectoryUtils {
    /// Liste les dossiers d'analyse disponibles dans le répertoire configuré
    /// Par défaut dans /data/analysis, mais configurable
    pub fn list_analysis_directories(
        base_path: Option<&str>,
    ) -> Result<Vec<DirectoryInfo>, ModelError> {
        let analysis_path = base_path.unwrap_or("/data/analysis");
        Self::list_directories(analysis_path)
    }

    /// Liste les dossiers ONT disponibles dans /data/raw/sequenceur/GRIDION
    pub fn list_ont_directories() -> Result<Vec<DirectoryInfo>, ModelError> {
        let ont_path = "/data/raw/sequenceur/GRIDION";
        Self::list_directories(ont_path)
    }

    /// Fonction générique pour lister les dossiers dans un répertoire donné
    fn list_directories(base_path: &str) -> Result<Vec<DirectoryInfo>, ModelError> {
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

    /// Valide qu'un chemin de dossier existe et est accessible
    pub fn validate_directory_path(path: &str) -> Result<bool, ModelError> {
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Ok(false);
        }
        if !path_obj.is_dir() {
            return Ok(false);
        }
        // Vérifier les permissions de lecture
        match fs::read_dir(path) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_list_directories() {
        // Créer un répertoire temporaire pour les tests
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Créer quelques sous-répertoires de test
        let sub_dir1 = temp_path.join("test_dir1");
        let sub_dir2 = temp_path.join("test_dir2");
        let _file = temp_path.join("test_file.txt");

        fs::create_dir_all(&sub_dir1).unwrap();
        fs::create_dir_all(&sub_dir2).unwrap();
        fs::write(&_file, "test content").unwrap();

        // Tester la fonction list_directories
        let result = DirectoryUtils::list_directories(temp_path.to_str().unwrap()).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|d| d.name == "test_dir1"));
        assert!(result.iter().any(|d| d.name == "test_dir2"));
    }

    #[test]
    fn test_validate_directory_path() {
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Tester un répertoire existant
        assert!(DirectoryUtils::validate_directory_path(temp_path.to_str().unwrap()).unwrap());

        // Tester un répertoire inexistant
        let non_existent = temp_path.join("non_existent");
        assert!(!DirectoryUtils::validate_directory_path(non_existent.to_str().unwrap()).unwrap());
    }
}
