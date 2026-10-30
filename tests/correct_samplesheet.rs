use smaug::utils::correct_samplesheet;
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
    // We expect the cleaning to have modified the original input (spaces/diacritics)
    assert_ne!(cleaned, input);
    assert!(cleaned.contains("echantillon1"));
}

#[test]
fn invalid_samplesheet_missing_sample_id() {
    let input = fs::read_to_string("tests/samples/invalid.csv").unwrap();
    let res = correct_samplesheet(&input);
    assert!(res.is_err());
    let err = res.err().unwrap().to_string();
    // The expected error contains an obvious mention in French
    assert!(err.to_lowercase().contains("aucune") || err.to_lowercase().contains("impossible"));
}
