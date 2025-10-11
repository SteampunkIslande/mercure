use chrono::{Datelike, NaiveDate};
use diacritics::remove_diacritics;
use std::collections::HashMap;
use std::io::BufRead;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UtilsError {
    #[error("Erreur dans la samplesheet:{0}")]
    SampleSheetError(String),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

/// Fonction utilitaire pour nettoyer une chaîne de caractères
fn clean_string(input: &str) -> String {
    let no_spaces = input.replace(' ', "").replace("\t", "");
    remove_diacritics(&no_spaces)
}

pub fn parse_sample_sheet(
    path: &PathBuf,
) -> Result<(String, HashMap<String, Vec<String>>), UtilsError> {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();
    let reader = std::fs::File::open(path).map(std::io::BufReader::new)?;
    let mut column_names: Vec<String> = Vec::new();
    let mut clean_column_names: Vec<String> = Vec::new();
    let mut lines = reader.lines();
    let mut in_data_section = false;
    let mut cleaned_samplesheet_lines = Vec::new();

    while let Some(line) = lines.next() {
        let line = line?;
        let line = line.trim();

        // Ignorer les lignes vides
        if line.is_empty() {
            continue;
        }

        // Si on n'est pas encore dans la section Data, copier la ligne telle quelle
        if !in_data_section && line != "[Data]" {
            cleaned_samplesheet_lines.push(line.to_string());
            continue;
        }

        // Détecter le début de la section [Data]
        if line == "[Data]" {
            in_data_section = true;
            cleaned_samplesheet_lines.push(line.to_string());

            // Lire la ligne suivante qui contient les noms de colonnes
            if let Some(header_line) = lines.next() {
                let header_line = header_line?;
                column_names = header_line
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                // Créer les noms de colonnes nettoyés pour la samplesheet de sortie
                clean_column_names = column_names.iter().map(|col| clean_string(col)).collect();

                // Ajouter l'en-tête nettoyé à la samplesheet de sortie
                cleaned_samplesheet_lines.push(clean_column_names.join(","));

                // Initialiser le HashMap avec des vecteurs vides pour chaque colonne originale
                for column_name in &column_names {
                    result.insert(column_name.clone(), Vec::new());
                }
            } else {
                return Err(UtilsError::SampleSheetError(
                    "Aucune ligne d'en-tête trouvée après [Data]".to_string(),
                ));
            }
            continue;
        }

        // Si on est dans la section Data et qu'on a des noms de colonnes, traiter les données
        if in_data_section && !column_names.is_empty() {
            let values: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

            // Vérifier que le nombre de valeurs correspond au nombre de colonnes
            if values.len() != column_names.len() {
                return Err(UtilsError::SampleSheetError(format!(
                    "Nombre de valeurs ({}) ne correspond pas au nombre de colonnes ({})",
                    values.len(),
                    column_names.len()
                )));
            }

            // Ajouter chaque valeur au HashMap (valeurs originales)
            for (i, value) in values.iter().enumerate() {
                if let Some(column_vec) = result.get_mut(&column_names[i]) {
                    column_vec.push(value.to_string());
                }
            }

            // Ajouter la ligne nettoyée à la samplesheet de sortie
            let cleaned_values: Vec<String> = values.iter().map(|v| clean_string(v)).collect();
            cleaned_samplesheet_lines.push(cleaned_values.join(","));
        }
    }

    // Vérifier qu'on a trouvé au moins la section Data
    if column_names.is_empty() {
        return Err(UtilsError::SampleSheetError(
            "Section [Data] non trouvée dans le samplesheet".to_string(),
        ));
    }

    // Créer la samplesheet nettoyée
    let cleaned_samplesheet = cleaned_samplesheet_lines.join("\n");

    Ok((cleaned_samplesheet, result))
}

pub fn format_french_date(date_str: &str) -> String {
    // Extraire juste la partie date si c'est un datetime
    let date_part = date_str.split('T').next().unwrap_or(date_str);

    if let Ok(date) = NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
        let weekday = match date.weekday() {
            chrono::Weekday::Mon => "lun.",
            chrono::Weekday::Tue => "mar.",
            chrono::Weekday::Wed => "mer.",
            chrono::Weekday::Thu => "jeu.",
            chrono::Weekday::Fri => "ven.",
            chrono::Weekday::Sat => "sam.",
            chrono::Weekday::Sun => "dim.",
        };

        let month_name = match date.month() {
            1 => "jan.",
            2 => "fév.",
            3 => "mars",
            4 => "avr.",
            5 => "mai",
            6 => "juin",
            7 => "jui.",
            8 => "août",
            9 => "sept.",
            10 => "oct.",
            11 => "nov.",
            12 => "déc.",
            _ => "creepy",
        };

        format!("{} {} {} {}", weekday, date.day(), month_name, date.year())
    } else {
        // Fallback si le parsing échoue
        date_str.to_string()
    }
}
