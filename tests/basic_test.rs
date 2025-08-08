// Basic integration test for the Rusty BLS Data Processing library

use rusty::{config, data, processing, output, utils};

#[test]
fn test_version() {
    // Test that the version constant is defined
    assert!(!rusty::VERSION.is_empty());
}

#[test]
fn test_add() {
    // Test the simple add function
    assert_eq!(rusty::add(2, 3), 5);
    assert_eq!(rusty::add(-1, 1), 0);
    assert_eq!(rusty::add(0, 0), 0);
}

#[test]
fn test_parse_bls_value() {
    // Test the parse_bls_value function
    assert_eq!(rusty::parse_bls_value("123.456").unwrap(), 123.456);
    assert_eq!(rusty::parse_bls_value("0").unwrap(), 0.0);
    assert!(rusty::parse_bls_value("invalid").is_err());
}

#[test]
fn test_utils_title_case() {
    // Test the to_title_case utility function
    assert_eq!(utils::to_title_case("hello world"), "Hello World");
    assert_eq!(utils::to_title_case("HELLO WORLD"), "Hello World");
    assert_eq!(utils::to_title_case("hello-world"), "Hello-World");
}

#[test]
fn test_utils_survey_code_validation() {
    // Test the is_valid_survey_code utility function
    assert!(utils::is_valid_survey_code("ap"));
    assert!(utils::is_valid_survey_code("AP"));
    assert!(!utils::is_valid_survey_code("a"));
    assert!(!utils::is_valid_survey_code("123"));
    assert!(!utils::is_valid_survey_code("a1"));
}

#[test]
fn test_utils_format_file_size() {
    // Test the format_file_size utility function
    assert_eq!(utils::format_file_size(0), "0 bytes");
    assert_eq!(utils::format_file_size(1023), "1023 bytes");
    assert_eq!(utils::format_file_size(1024), "1.00 KB");
    assert_eq!(utils::format_file_size(1024 * 1024), "1.00 MB");
    assert_eq!(utils::format_file_size(1024 * 1024 * 1024), "1.00 GB");
}