use mercure::utils::correct_samplesheet;
use std::fs;

#[test]
fn valid_samplesheet() {
    let input = fs::read_to_string("tests/samples/valid.csv").unwrap();
    let res = correct_samplesheet(&input);
    assert!(res.is_ok());
    let (samples, cleaned) = res.unwrap();
    assert_eq!(samples, vec!["sample1".to_string(), "sample2".to_string()]);
    assert_eq!(cleaned, input);
}

#[test]
fn diacritics_and_spaces() {
    let input = fs::read_to_string("tests/samples/diacritics.csv").unwrap();
    let res = correct_samplesheet(&input);
    assert!(res.is_ok());
    let (samples, cleaned) = res.unwrap();
    assert_eq!(
        samples,
        vec!["echantillon1".to_string(), "sample2".to_string()]
    );
    // On attend que le nettoyage ait modifié l'entrée originale (espaces/diacritiques)
    assert_ne!(cleaned, input);
    assert!(cleaned.contains("echantillon1"));
}

#[test]
fn invalid_samplesheet_missing_sample_id() {
    let input = fs::read_to_string("tests/samples/invalid.csv").unwrap();
    let res = correct_samplesheet(&input);
    assert!(res.is_err());
    let err = res.err().unwrap();
    // L'erreur attendue contient une mention évidente en français
    assert!(err.to_lowercase().contains("aucune") || err.to_lowercase().contains("impossible"));
}
