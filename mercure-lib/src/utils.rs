use std::collections::HashMap;

use lazy_regex::{Lazy, Regex, lazy_regex};

use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use diacritics::remove_diacritics;

/// Fonction utilitaire pour nettoyer une chaîne de caractères
fn clean_string(input: &str) -> String {
    // Supprimer tous les caractères d'espacement (espaces, tabulations, retours, etc.)
    let no_whitespace: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    remove_diacritics(&no_whitespace)
}

pub fn correct_samplesheet(input: &str) -> Result<(Vec<String>, String), String> {
    // Séparer en lignes en gardant l'ordre
    let lines: Vec<&str> = input.lines().collect();
    let mut cleaned_lines: Vec<String> = Vec::new();
    let mut in_data = false;
    let mut column_names: Vec<String> = Vec::new();
    let mut sample_col_index: Option<usize> = None;
    let mut samples: Vec<String> = Vec::new();

    let mut i: usize = 0;
    while i < lines.len() {
        let line = lines[i];

        // Conserver les lignes vides telles quelles
        if line.is_empty() {
            cleaned_lines.push(line.to_string());
            i += 1;
            continue;
        }

        // Avant la section [Data], conserver telles quelles
        if !in_data && line != "[Data]" {
            cleaned_lines.push(line.to_string());
            i += 1;
            continue;
        }

        // Début de la section [Data]
        if line == "[Data]" {
            in_data = true;
            cleaned_lines.push(line.to_string());

            // Trouver la première ligne non vide qui suit comme en-tête
            i += 1;
            let mut header_line_opt: Option<&str> = None;
            while i < lines.len() {
                if lines[i].is_empty() {
                    cleaned_lines.push(lines[i].to_string());
                    i += 1;
                    continue;
                }
                header_line_opt = Some(lines[i]);
                break;
            }

            let header_line = match header_line_opt {
                Some(h) => h,
                None => return Err("Aucune ligne d'en-tête trouvée après [Data]".to_string()),
            };

            column_names = header_line.split(',').map(|s| s.to_string()).collect();

            // En-tête nettoyée
            let clean_column_names: Vec<String> =
                column_names.iter().map(|c| clean_string(c)).collect();
            cleaned_lines.push(clean_column_names.join(","));

            // Déterminer l'index de la colonne "sample"

            sample_col_index = column_names.iter().position(|col| col == "Sample_ID");

            if sample_col_index.is_none() {
                return Err("Impossible d'extraire la liste des échantillons: aucune colonne 'sample' trouvée"
                    .to_string());
            }

            // Passer à la ligne suivant l'en-tête pour lire les données
            i += 1;
            continue;
        }

        // Dans la section Data : traiter les lignes jusqu'à la fin ou une nouvelle section
        if in_data && !column_names.is_empty() {
            if line.starts_with('[') {
                // nouvelle section rencontrée : sortir du mode Data et conserver la ligne
                in_data = false;
                cleaned_lines.push(line.to_string());
                i += 1;
                continue;
            }

            let values: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if values.len() != column_names.len() {
                return Err(format!(
                    "Le nombre de colonnes diffère à la ligne {}",
                    i + 1
                ));
            }

            // Construire une ligne nettoyée en respectant le nombre de colonnes (pad si nécessaire)
            let mut cleaned_values: Vec<String> = Vec::new();
            for col_idx in 0..column_names.len() {
                let v = values.get(col_idx).copied().unwrap_or("");
                cleaned_values.push(clean_string(v));
            }
            cleaned_lines.push(cleaned_values.join(","));

            // Extraire l'échantillon si présent
            if let Some(sample_idx) = sample_col_index
                && let Some(val) = values.get(sample_idx)
            {
                let cleaned_sample = clean_string(val);
                if !cleaned_sample.is_empty() {
                    samples.push(cleaned_sample);
                }
            }

            i += 1;
            continue;
        }

        // Cas général : avancer
        cleaned_lines.push(line.to_string());
        i += 1;
    }

    if samples.is_empty() {
        return Err(
            "Impossible d'extraire la liste des échantillons: aucune valeur trouvée".to_string(),
        );
    }

    let cleaned = cleaned_lines.join("\n");
    Ok((samples, cleaned))
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
            1 => "janv.",
            2 => "fév.",
            3 => "mars",
            4 => "avr.",
            5 => "mai",
            6 => "juin",
            7 => "juill.",
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

static VAR_REGEX: Lazy<Regex> = lazy_regex!(r"(?m)^##\s*(\S+)\s+(.+)$");

pub fn parse_launcher(launcher_content: &str) -> HashMap<String, String> {
    // In the launcher content, look for lines starting with ## VAR_NAME description
    let mut vars: HashMap<String, String> = HashMap::new();

    for cap in VAR_REGEX.captures_iter(launcher_content) {
        if let (Some(var), Some(desc)) = (cap.get(1), cap.get(2)) {
            let var_name = var.as_str().to_string();
            vars.insert(var_name, desc.as_str().to_string());
        }
    }

    vars
}

/// Convertit le nom d'un fichier local en un nom servi par la route statique
/// # Arguments
/// * `filename` - Le chemin complet du fichier local (sur le serveur)
/// * `base_path` - Le chemin de base à retirer (le nom du répertoire de téléchargement sur le serveur, servi statiquement)
/// * `prefix` - Le préfixe (en chemin absolu) utilisé dans la route statique (ex: "/uploads")
pub fn filename_to_static_served_name(
    filename: &str,
    base_path: &str,
    prefix: &str,
) -> Option<String> {
    let path = std::path::Path::new(filename).canonicalize().ok()?;
    path.strip_prefix(base_path)
        .ok()
        .map(|rel_path| format!("{}/{}", prefix, rel_path.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_launcher() {
        let launcher_content = r#"
## VAR1 Description of var1
## VAR2 Description of var2
Some other content
## VAR3 Description of var3
        "#;

        let vars = parse_launcher(launcher_content);
        assert_eq!(vars.get("VAR1"), Some(&"Description of var1".to_string()));
        assert_eq!(vars.get("VAR2"), Some(&"Description of var2".to_string()));
        assert_eq!(vars.get("VAR3"), Some(&"Description of var3".to_string()));
        assert_eq!(vars.len(), 3);
    }
}
