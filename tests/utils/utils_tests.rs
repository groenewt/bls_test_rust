//! Comprehensive unit tests for utility functions
//! 
//! These tests verify the utility system including:
//! - Path manipulation and validation utilities
//! - File I/O operations and helpers
//! - Time formatting and parsing utilities
//! - Data formatting and conversion utilities
//! - Data validation helpers and rules
//! - String manipulation and formatting
//! - Configuration path resolution
//! 
//! The tests cover:
//! - Path construction and validation
//! - File operations and error handling
//! - Time parsing and formatting edge cases
//! - BLS data format validation
//! - Survey code validation and normalization
//! - File size formatting and parsing
//! - Configuration directory resolution
//! - Cross-platform compatibility

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use tempfile::TempDir;
use chrono::{DateTime, Utc, NaiveDateTime, TimeZone};

use rusty::utils::{
    PathUtils, get_config_path, get_data_path, get_output_path,
    FileUtils, read_to_string, write_string, ensure_directory,
    TimeUtils, current_timestamp, format_timestamp, parse_timestamp,
    FormatUtils, format_bls_value, parse_bls_value, format_survey_code,
    ValidationUtils, validate_survey_code, validate_file_path, validate_config,
    to_title_case, is_valid_survey_code, format_file_size
};
use rusty::error::{Result, SystemError};

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn test_path_utils_creation() {
        let path_utils = PathUtils::new();
        assert!(path_utils.base_path().is_none());
    }

    #[test]
    fn test_path_utils_with_base() {
        let base_path = PathBuf::from("/home/user/rusty");
        let path_utils = PathUtils::new().with_base_path(base_path.clone());
        assert_eq!(path_utils.base_path(), Some(&base_path));
    }

    #[test]
    fn test_get_config_path() {
        let config_path = get_config_path("surveys", "ap.yml").unwrap();
        
        assert!(config_path.to_string_lossy().contains("config"));
        assert!(config_path.to_string_lossy().contains("surveys"));
        assert!(config_path.to_string_lossy().contains("ap.yml"));
    }

    #[test]
    fn test_get_config_path_nested() {
        let config_path = get_config_path("surveys/AP", "overview.yml").unwrap();
        
        assert!(config_path.to_string_lossy().contains("config"));
        assert!(config_path.to_string_lossy().contains("surveys"));
        assert!(config_path.to_string_lossy().contains("AP"));
        assert!(config_path.to_string_lossy().contains("overview.yml"));
    }

    #[test]
    fn test_get_data_path() {
        let data_path = get_data_path("raw/bls", "ap.series").unwrap();
        
        assert!(data_path.to_string_lossy().contains("data"));
        assert!(data_path.to_string_lossy().contains("raw"));
        assert!(data_path.to_string_lossy().contains("bls"));
        assert!(data_path.to_string_lossy().contains("ap.series"));
    }

    #[test]
    fn test_get_output_path() {
        let output_path = get_output_path("processed", "ap_series.csv").unwrap();
        
        assert!(output_path.to_string_lossy().contains("data"));
        assert!(output_path.to_string_lossy().contains("processed"));
        assert!(output_path.to_string_lossy().contains("ap_series.csv"));
    }

    #[test]
    fn test_path_validation() {
        let path_utils = PathUtils::new();
        
        // Valid paths
        assert!(path_utils.is_valid_path("config/surveys/ap.yml"));
        assert!(path_utils.is_valid_path("data/raw/bls/ap.series"));
        assert!(path_utils.is_valid_path("output/processed/ap_data.csv"));
        
        // Invalid paths
        assert!(!path_utils.is_valid_path(""));
        assert!(!path_utils.is_valid_path("../../../etc/passwd"));
        assert!(!path_utils.is_valid_path("config/../../../secret"));
    }

    #[test]
    fn test_path_normalization() {
        let path_utils = PathUtils::new();
        
        let normalized = path_utils.normalize_path("config//surveys/../surveys/ap.yml");
        assert_eq!(normalized.to_string_lossy(), "config/surveys/ap.yml");
        
        let normalized = path_utils.normalize_path("./data/raw/bls/ap.series");
        assert_eq!(normalized.to_string_lossy(), "data/raw/bls/ap.series");
    }

    #[test]
    fn test_path_extension_handling() {
        let path_utils = PathUtils::new();
        
        assert_eq!(path_utils.get_extension("config.yml"), Some("yml"));
        assert_eq!(path_utils.get_extension("data.csv.gz"), Some("gz"));
        assert_eq!(path_utils.get_extension("no_extension"), None);
        assert_eq!(path_utils.get_extension(""), None);
    }

    #[test]
    fn test_path_joining() {
        let path_utils = PathUtils::new();
        
        let joined = path_utils.join_paths(&["config", "surveys", "ap.yml"]);
        assert!(joined.to_string_lossy().contains("config"));
        assert!(joined.to_string_lossy().contains("surveys"));
        assert!(joined.to_string_lossy().contains("ap.yml"));
    }

    #[test]
    fn test_relative_path_resolution() {
        let path_utils = PathUtils::new();
        
        let resolved = path_utils.resolve_relative_path("config", "../data/raw");
        assert!(resolved.to_string_lossy().contains("data"));
        assert!(resolved.to_string_lossy().contains("raw"));
    }
}

#[cfg(test)]
mod file_tests {
    use super::*;

    #[test]
    fn test_file_utils_creation() {
        let file_utils = FileUtils::new();
        assert!(file_utils.default_encoding().contains("utf"));
    }

    #[test]
    fn test_ensure_directory() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path().join("test_directory");
        
        assert!(!test_dir.exists());
        
        let result = ensure_directory(&test_dir);
        assert!(result.is_ok());
        assert!(test_dir.exists());
        assert!(test_dir.is_dir());
    }

    #[test]
    fn test_ensure_nested_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("level1").join("level2").join("level3");
        
        assert!(!nested_dir.exists());
        
        let result = ensure_directory(&nested_dir);
        assert!(result.is_ok());
        assert!(nested_dir.exists());
        assert!(nested_dir.is_dir());
    }

    #[test]
    fn test_write_and_read_string() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        
        let content = "Hello, World!\nThis is a test file.";
        
        // Write content
        let write_result = write_string(&test_file, content);
        assert!(write_result.is_ok());
        assert!(test_file.exists());
        
        // Read content back
        let read_result = read_to_string(&test_file);
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), content);
    }

    #[test]
    fn test_write_string_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_file = temp_dir.path().join("nested").join("dir").join("test.txt");
        
        let content = "Test content";
        
        // Write should create the directory structure
        let result = write_string(&nested_file, content);
        assert!(result.is_ok());
        assert!(nested_file.exists());
        assert!(nested_file.parent().unwrap().exists());
    }

    #[test]
    fn test_read_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_file = temp_dir.path().join("nonexistent.txt");
        
        let result = read_to_string(&nonexistent_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_operations_with_unicode() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("unicode_test.txt");
        
        let content = "Hello, 世界! 🌍 Café naïve résumé";
        
        let write_result = write_string(&test_file, content);
        assert!(write_result.is_ok());
        
        let read_result = read_to_string(&test_file);
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), content);
    }

    #[test]
    fn test_file_utils_methods() {
        let file_utils = FileUtils::new();
        
        // Test file size calculation
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("size_test.txt");
        let content = "0123456789"; // 10 bytes
        
        write_string(&test_file, content).unwrap();
        
        let size = file_utils.get_file_size(&test_file).unwrap();
        assert_eq!(size, 10);
    }

    #[test]
    fn test_file_exists_check() {
        let file_utils = FileUtils::new();
        let temp_dir = TempDir::new().unwrap();
        
        let existing_file = temp_dir.path().join("exists.txt");
        let nonexistent_file = temp_dir.path().join("does_not_exist.txt");
        
        write_string(&existing_file, "content").unwrap();
        
        assert!(file_utils.file_exists(&existing_file));
        assert!(!file_utils.file_exists(&nonexistent_file));
    }

    #[test]
    fn test_file_backup_creation() {
        let file_utils = FileUtils::new();
        let temp_dir = TempDir::new().unwrap();
        let original_file = temp_dir.path().join("original.txt");
        
        write_string(&original_file, "original content").unwrap();
        
        let backup_result = file_utils.create_backup(&original_file);
        assert!(backup_result.is_ok());
        
        let backup_path = backup_result.unwrap();
        assert!(backup_path.exists());
        assert!(backup_path.to_string_lossy().contains("backup"));
    }
}

#[cfg(test)]
mod time_tests {
    use super::*;

    #[test]
    fn test_time_utils_creation() {
        let time_utils = TimeUtils::new();
        assert_eq!(time_utils.default_format(), "%Y-%m-%d %H:%M:%S");
    }

    #[test]
    fn test_current_timestamp() {
        let timestamp1 = current_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let timestamp2 = current_timestamp();
        
        assert!(timestamp2 > timestamp1);
        assert!(timestamp1.len() > 0);
        assert!(timestamp2.len() > 0);
    }

    #[test]
    fn test_format_timestamp() {
        let dt = Utc.with_ymd_and_hms(2023, 12, 25, 15, 30, 45).unwrap();
        
        let formatted = format_timestamp(&dt, "%Y-%m-%d %H:%M:%S");
        assert_eq!(formatted, "2023-12-25 15:30:45");
        
        let formatted_iso = format_timestamp(&dt, "%Y-%m-%dT%H:%M:%SZ");
        assert_eq!(formatted_iso, "2023-12-25T15:30:45Z");
    }

    #[test]
    fn test_parse_timestamp() {
        let timestamp_str = "2023-12-25 15:30:45";
        let format = "%Y-%m-%d %H:%M:%S";
        
        let parsed = parse_timestamp(timestamp_str, format);
        assert!(parsed.is_ok());
        
        let dt = parsed.unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 25);
        assert_eq!(dt.hour(), 15);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        let invalid_timestamps = vec![
            "invalid-date",
            "2023-13-01 00:00:00", // Invalid month
            "2023-12-32 00:00:00", // Invalid day
            "2023-12-25 25:00:00", // Invalid hour
        ];
        
        for invalid_ts in invalid_timestamps {
            let result = parse_timestamp(invalid_ts, "%Y-%m-%d %H:%M:%S");
            assert!(result.is_err(), "Should fail to parse: {}", invalid_ts);
        }
    }

    #[test]
    fn test_time_utils_formats() {
        let time_utils = TimeUtils::new();
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 9, 30, 0).unwrap();
        
        // Test various formats
        let iso_format = time_utils.format_iso(&dt);
        assert!(iso_format.contains("2023-06-15"));
        assert!(iso_format.contains("09:30:00"));
        
        let date_only = time_utils.format_date_only(&dt);
        assert_eq!(date_only, "2023-06-15");
        
        let time_only = time_utils.format_time_only(&dt);
        assert_eq!(time_only, "09:30:00");
    }

    #[test]
    fn test_time_zone_handling() {
        let time_utils = TimeUtils::new();
        let dt = Utc.with_ymd_and_hms(2023, 6, 15, 12, 0, 0).unwrap();
        
        // Test UTC formatting
        let utc_formatted = time_utils.format_utc(&dt);
        assert!(utc_formatted.contains("UTC") || utc_formatted.contains("Z"));
        
        // Test local time conversion (this might vary by system)
        let local_formatted = time_utils.format_local(&dt);
        assert!(!local_formatted.is_empty());
    }

    #[test]
    fn test_duration_formatting() {
        let time_utils = TimeUtils::new();
        
        let duration_ms = 1500; // 1.5 seconds
        let formatted = time_utils.format_duration_ms(duration_ms);
        assert!(formatted.contains("1.5") || formatted.contains("1500"));
        
        let duration_secs = 125; // 2 minutes 5 seconds
        let formatted = time_utils.format_duration_seconds(duration_secs);
        assert!(formatted.contains("2") && formatted.contains("5"));
    }
}

#[cfg(test)]
mod format_tests {
    use super::*;

    #[test]
    fn test_format_utils_creation() {
        let format_utils = FormatUtils::new();
        assert_eq!(format_utils.default_decimal_places(), 2);
    }

    #[test]
    fn test_format_bls_value() {
        // Test normal values
        assert_eq!(format_bls_value(123.456).unwrap(), "123.46");
        assert_eq!(format_bls_value(0.0).unwrap(), "0.00");
        assert_eq!(format_bls_value(-45.789).unwrap(), "-45.79");
        
        // Test edge cases
        assert_eq!(format_bls_value(f64::INFINITY).unwrap(), "∞");
        assert_eq!(format_bls_value(f64::NEG_INFINITY).unwrap(), "-∞");
        assert!(format_bls_value(f64::NAN).is_err());
    }

    #[test]
    fn test_parse_bls_value() {
        // Test normal values
        assert_eq!(parse_bls_value("123.456").unwrap(), 123.456);
        assert_eq!(parse_bls_value("0").unwrap(), 0.0);
        assert_eq!(parse_bls_value("-45.789").unwrap(), -45.789);
        assert_eq!(parse_bls_value("  123.45  ").unwrap(), 123.45); // Whitespace
        
        // Test invalid values
        assert!(parse_bls_value("invalid").is_err());
        assert!(parse_bls_value("").is_err());
        assert!(parse_bls_value("123.45.67").is_err());
    }

    #[test]
    fn test_format_survey_code() {
        assert_eq!(format_survey_code("ap"), "AP");
        assert_eq!(format_survey_code("AP"), "AP");
        assert_eq!(format_survey_code("bd"), "BD");
        assert_eq!(format_survey_code("Ce"), "CE");
    }

    #[test]
    fn test_format_survey_code_invalid() {
        // Invalid survey codes should return error or be handled gracefully
        let result = format_survey_code("a"); // Too short
        match result {
            Ok(code) => assert_eq!(code.len(), 2), // Might be padded
            Err(_) => {} // Or might return error
        }
        
        let result = format_survey_code("abc"); // Too long
        match result {
            Ok(code) => assert_eq!(code.len(), 2), // Might be truncated
            Err(_) => {} // Or might return error
        }
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 bytes");
        assert_eq!(format_file_size(1), "1 byte");
        assert_eq!(format_file_size(1023), "1023 bytes");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1536), "1.50 KB"); // 1.5 KB
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_file_size(1024_u64.pow(4)), "1.00 TB");
    }

    #[test]
    fn test_format_percentage() {
        let format_utils = FormatUtils::new();
        
        assert_eq!(format_utils.format_percentage(0.0), "0.00%");
        assert_eq!(format_utils.format_percentage(0.5), "50.00%");
        assert_eq!(format_utils.format_percentage(1.0), "100.00%");
        assert_eq!(format_utils.format_percentage(0.12345), "12.35%");
    }

    #[test]
    fn test_format_currency() {
        let format_utils = FormatUtils::new();
        
        assert_eq!(format_utils.format_currency(123.45), "$123.45");
        assert_eq!(format_utils.format_currency(0.0), "$0.00");
        assert_eq!(format_utils.format_currency(-50.25), "-$50.25");
        assert_eq!(format_utils.format_currency(1234567.89), "$1,234,567.89");
    }

    #[test]
    fn test_format_number_with_commas() {
        let format_utils = FormatUtils::new();
        
        assert_eq!(format_utils.format_with_commas(1234), "1,234");
        assert_eq!(format_utils.format_with_commas(1234567), "1,234,567");
        assert_eq!(format_utils.format_with_commas(0), "0");
        assert_eq!(format_utils.format_with_commas(123), "123");
    }

    #[test]
    fn test_to_title_case() {
        assert_eq!(to_title_case("hello world"), "Hello World");
        assert_eq!(to_title_case("HELLO WORLD"), "Hello World");
        assert_eq!(to_title_case("hello-world"), "Hello-World");
        assert_eq!(to_title_case("hello_world"), "Hello_World");
        assert_eq!(to_title_case(""), "");
        assert_eq!(to_title_case("a"), "A");
    }

    #[test]
    fn test_string_truncation() {
        let format_utils = FormatUtils::new();
        
        assert_eq!(format_utils.truncate("hello world", 5), "hello");
        assert_eq!(format_utils.truncate("hello", 10), "hello");
        assert_eq!(format_utils.truncate("hello world", 8), "hello...");
        assert_eq!(format_utils.truncate("", 5), "");
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_validation_utils_creation() {
        let validation_utils = ValidationUtils::new();
        assert!(validation_utils.strict_mode() == false); // Default should be lenient
    }

    #[test]
    fn test_is_valid_survey_code() {
        // Valid survey codes
        assert!(is_valid_survey_code("AP"));
        assert!(is_valid_survey_code("BD"));
        assert!(is_valid_survey_code("CE"));
        assert!(is_valid_survey_code("CU"));
        
        // Case insensitive
        assert!(is_valid_survey_code("ap"));
        assert!(is_valid_survey_code("bd"));
        
        // Invalid survey codes
        assert!(!is_valid_survey_code("A")); // Too short
        assert!(!is_valid_survey_code("ABC")); // Too long
        assert!(!is_valid_survey_code("A1")); // Contains number
        assert!(!is_valid_survey_code("1A")); // Starts with number
        assert!(!is_valid_survey_code("")); // Empty
        assert!(!is_valid_survey_code("A-")); // Contains special character
    }

    #[test]
    fn test_validate_survey_code() {
        let result = validate_survey_code("AP");
        assert!(result.is_ok());
        
        let result = validate_survey_code("ap");
        assert!(result.is_ok());
        
        let result = validate_survey_code("A");
        assert!(result.is_err());
        
        let result = validate_survey_code("123");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_file_path() {
        // Valid paths
        assert!(validate_file_path("config/surveys/ap.yml").is_ok());
        assert!(validate_file_path("data/raw/bls/ap.series").is_ok());
        assert!(validate_file_path("output/processed/ap_data.csv").is_ok());
        
        // Invalid paths
        assert!(validate_file_path("").is_err());
        assert!(validate_file_path("../../../etc/passwd").is_err());
        assert!(validate_file_path("/absolute/path").is_err()); // Might be invalid depending on policy
    }

    #[test]
    fn test_validate_series_id() {
        let validation_utils = ValidationUtils::new();
        
        // Valid BLS series IDs
        assert!(validation_utils.validate_series_id("APUS49074714").is_ok());
        assert!(validation_utils.validate_series_id("BDUS00000001").is_ok());
        assert!(validation_utils.validate_series_id("CEUS0000SA0").is_ok());
        
        // Invalid series IDs
        assert!(validation_utils.validate_series_id("").is_err());
        assert!(validation_utils.validate_series_id("AP").is_err()); // Too short
        assert!(validation_utils.validate_series_id("APUS49074714TOOLONG").is_err()); // Too long
        assert!(validation_utils.validate_series_id("ap12345678").is_err()); // Lowercase
    }

    #[test]
    fn test_validate_year() {
        let validation_utils = ValidationUtils::new();
        
        // Valid years
        assert!(validation_utils.validate_year(2023).is_ok());
        assert!(validation_utils.validate_year(1990).is_ok());
        assert!(validation_utils.validate_year(2030).is_ok());
        
        // Invalid years
        assert!(validation_utils.validate_year(1800).is_err()); // Too old
        assert!(validation_utils.validate_year(2200).is_err()); // Too far in future
        assert!(validation_utils.validate_year(0).is_err());
    }

    #[test]
    fn test_validate_period() {
        let validation_utils = ValidationUtils::new();
        
        // Valid periods
        let valid_periods = vec!["M01", "M12", "Q01", "Q04", "A01", "S01", "S02"];
        for period in valid_periods {
            assert!(validation_utils.validate_period(period).is_ok(), 
                   "Period {} should be valid", period);
        }
        
        // Invalid periods
        let invalid_periods = vec!["M00", "M13", "Q00", "Q05", "X01", "", "INVALID"];
        for period in invalid_periods {
            assert!(validation_utils.validate_period(period).is_err(), 
                   "Period {} should be invalid", period);
        }
    }

    #[test]
    fn test_validate_observation_value() {
        let validation_utils = ValidationUtils::new();
        
        // Valid values
        assert!(validation_utils.validate_observation_value(Some(123.45)).is_ok());
        assert!(validation_utils.validate_observation_value(Some(0.0)).is_ok());
        assert!(validation_utils.validate_observation_value(Some(-100.0)).is_ok());
        assert!(validation_utils.validate_observation_value(None).is_ok()); // Null values allowed
        
        // Invalid values (depending on implementation)
        assert!(validation_utils.validate_observation_value(Some(f64::NAN)).is_err());
        assert!(validation_utils.validate_observation_value(Some(f64::INFINITY)).is_err());
        assert!(validation_utils.validate_observation_value(Some(f64::NEG_INFINITY)).is_err());
    }

    #[test]
    fn test_validate_config() {
        let mut config = HashMap::new();
        config.insert("survey_code".to_string(), "AP".to_string());
        config.insert("name".to_string(), "Average Price Data".to_string());
        config.insert("version".to_string(), "1.0".to_string());
        
        let result = validate_config(&config);
        assert!(result.is_ok());
        
        // Test missing required field
        let mut incomplete_config = HashMap::new();
        incomplete_config.insert("name".to_string(), "Test".to_string());
        
        let result = validate_config(&incomplete_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_with_strict_mode() {
        let validation_utils = ValidationUtils::new().with_strict_mode(true);
        
        // In strict mode, validation should be more stringent
        assert!(validation_utils.strict_mode());
        
        // Test that strict mode affects validation
        let result = validation_utils.validate_series_id("apus49074714"); // lowercase
        assert!(result.is_err()); // Should fail in strict mode
    }

    #[test]
    fn test_email_validation() {
        let validation_utils = ValidationUtils::new();
        
        // Valid emails
        assert!(validation_utils.validate_email("user@example.com").is_ok());
        assert!(validation_utils.validate_email("test.email+tag@domain.co.uk").is_ok());
        
        // Invalid emails
        assert!(validation_utils.validate_email("invalid-email").is_err());
        assert!(validation_utils.validate_email("@domain.com").is_err());
        assert!(validation_utils.validate_email("user@").is_err());
        assert!(validation_utils.validate_email("").is_err());
    }

    #[test]
    fn test_url_validation() {
        let validation_utils = ValidationUtils::new();
        
        // Valid URLs
        assert!(validation_utils.validate_url("https://www.example.com").is_ok());
        assert!(validation_utils.validate_url("http://localhost:8080/path").is_ok());
        assert!(validation_utils.validate_url("ftp://files.example.com/file.txt").is_ok());
        
        // Invalid URLs
        assert!(validation_utils.validate_url("not-a-url").is_err());
        assert!(validation_utils.validate_url("").is_err());
        assert!(validation_utils.validate_url("://missing-scheme").is_err());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_path_file_integration() {
        let temp_dir = TempDir::new().unwrap();
        
        // Use path utilities to construct a path
        let config_path = get_config_path("test", "config.yml").unwrap();
        let full_path = temp_dir.path().join(config_path.file_name().unwrap());
        
        // Use file utilities to write and read
        let content = "test: configuration\nvalue: 123";
        write_string(&full_path, content).unwrap();
        
        let read_content = read_to_string(&full_path).unwrap();
        assert_eq!(read_content, content);
        
        // Use validation to check the path
        assert!(validate_file_path(full_path.to_str().unwrap()).is_ok());
    }

    #[test]
    fn test_time_format_integration() {
        let now = Utc::now();
        
        // Format timestamp
        let formatted = format_timestamp(&now, "%Y-%m-%d %H:%M:%S");
        
        // Parse it back
        let parsed = parse_timestamp(&formatted, "%Y-%m-%d %H:%M:%S").unwrap();
        
        // Should be very close (within a second due to precision)
        let diff = (now.timestamp() - parsed.timestamp()).abs();
        assert!(diff <= 1);
    }

    #[test]
    fn test_validation_format_integration() {
        // Test that validation and formatting work together
        let survey_code = "ap";
        
        // Validate (should pass)
        assert!(is_valid_survey_code(survey_code));
        
        // Format
        let formatted = format_survey_code(survey_code).unwrap();
        assert_eq!(formatted, "AP");
        
        // Validate formatted version
        assert!(is_valid_survey_code(&formatted));
    }

    #[test]
    fn test_bls_value_round_trip() {
        let original_values = vec![123.456, 0.0, -789.123, 1000000.789];
        
        for original in original_values {
            // Format the value
            let formatted = format_bls_value(original).unwrap();
            
            // Parse it back
            let parsed = parse_bls_value(&formatted).unwrap();
            
            // Should be equal within reasonable precision
            let diff = (original - parsed).abs();
            assert!(diff < 0.01, "Original: {}, Parsed: {}, Diff: {}", original, parsed, diff);
        }
    }

    #[test]
    fn test_comprehensive_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create a complex directory structure
        let survey_dir = temp_dir.path().join("config").join("surveys").join("AP");
        ensure_directory(&survey_dir).unwrap();
        
        // Write multiple configuration files
        let files = vec![
            ("overview.yml", "name: Average Price Data\nversion: 1.0"),
            ("model.yml", "series:\n  primary_key: series_id"),
            ("processing.yml", "strategy: in_memory\nmax_threads: 4"),
        ];
        
        for (filename, content) in files {
            let file_path = survey_dir.join(filename);
            write_string(&file_path, content).unwrap();
            
            // Verify file was written correctly
            let read_content = read_to_string(&file_path).unwrap();
            assert_eq!(read_content, content);
            
            // Validate the path
            assert!(validate_file_path(file_path.to_str().unwrap()).is_ok());
        }
        
        // Test file size formatting
        let overview_file = survey_dir.join("overview.yml");
        let file_utils = FileUtils::new();
        let size = file_utils.get_file_size(&overview_file).unwrap();
        let formatted_size = format_file_size(size);
        assert!(formatted_size.contains("bytes") || formatted_size.contains("KB"));
    }
}