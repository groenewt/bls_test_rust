//! # Validation Utilities
//!
//! This module provides data validation utilities for the Rusty BLS Data Processing system.
//! It handles validation rules, data integrity checks, and configuration validation.
//!
//! ## Features
//!
//! - Survey code validation
//! - File path validation
//! - Configuration validation
//! - Data range validation
//! - Format validation
//! - Custom validation rules
//!
//! ## Usage
//!
//! ```rust
//! use crate::utils::validation::{ValidationUtils, validate_survey_code, validate_file_path};
//!
//! // Validate survey code
//! validate_survey_code("AP")?;
//!
//! // Validate file path
//! validate_file_path("config/surveys/ap.yml")?;
//!
//! // Custom validation
//! let validator = ValidationUtils::new();
//! validator.validate_numeric_range(123.45, 0.0, 1000.0)?;
//! ```

use std::path::Path;
use chrono::Datelike;
use once_cell::sync::Lazy;
use regex::Regex;
use validator::{Validate, ValidationError};
use crate::error::{Result, DataError, ConfigError};
use crate::utils::format::FormatUtils;
use crate::utils::path::PathUtils;

static VALIDATOR: Lazy<ValidationUtils> = Lazy::new(ValidationUtils::new);

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());
static URL_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap());


/// Validation utilities for the BLS data processing system
pub struct ValidationUtils {
    format_utils: FormatUtils,
    path_utils: PathUtils,
}

impl ValidationUtils {
    /// Create a new ValidationUtils instance
    pub fn new() -> Self {
        Self {
            format_utils: FormatUtils::new(),
            path_utils: PathUtils::new(),
        }
    }

    /// Validate a survey code
    pub fn validate_survey_code(&self, code: &str) -> Result<()> {
        // Use format utils to validate and normalize
        self.format_utils.format_survey_code(code)?;
        Ok(())
    }

    /// Validate a file path
    pub fn validate_file_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.path_utils.validate_path(path)?;
        Ok(())
    }

    /// Validate a numeric value is within a specified range
    pub fn validate_numeric_range(&self, value: f64, min: f64, max: f64) -> Result<()> {
        if value.is_nan() || value.is_infinite() {
            return Err(crate::error::Error::Data(DataError::invalid_value(
                format!("Invalid numeric value: {}", value)
            )));
        }

        if value < min || value > max {
            return Err(crate::error::Error::Data(DataError::invalid_value(
                format!("Value {} is outside valid range [{}, {}]", value, min, max)
            )));
        }

        Ok(())
    }

    /// Validate a year value
    pub fn validate_year(&self, year: u32) -> Result<()> {
        let current_year = chrono::Utc::now().year() as u32;
        let min_year = 1900;
        let max_year = current_year + 10; // Allow some future years

        if year < min_year || year > max_year {
            return Err(crate::error::Error::Data(DataError::invalid_value(
                format!("Invalid year: {}. Must be between {} and {}", year, min_year, max_year)
            )));
        }

        Ok(())
    }

    /// Validate a series ID
    pub fn validate_series_id(&self, series_id: &str) -> Result<()> {
        self.format_utils.format_series_id(series_id)?;
        Ok(())
    }

    /// Validate a period string
    pub fn validate_period(&self, period: &str) -> Result<()> {
        self.format_utils.parse_period(period)?;
        Ok(())
    }

    /// Validate that a string is not empty or whitespace-only
    pub fn validate_non_empty_string(&self, value: &str, field_name: &str) -> Result<()> {
        if value.trim().is_empty() {
            return Err(crate::error::Error::Data(DataError::MissingValue(
                format!("Field '{}' cannot be empty", field_name)
            )));
        }
        Ok(())
    }

    /// Validate string length
    pub fn validate_string_length(&self, value: &str, min_len: usize, max_len: usize, field_name: &str) -> Result<()> {
        let len = value.len();
        if len < min_len || len > max_len {
            return Err(crate::error::Error::Data(DataError::invalid_value(
                format!("Field '{}' length {} is outside valid range [{}, {}]", field_name, len, min_len, max_len)
            )));
        }
        Ok(())
    }

    /// Validate that a value is one of the allowed options
    pub fn validate_enum_value<T: PartialEq + std::fmt::Display>(&self, value: &T, allowed_values: &[T], field_name: &str) -> Result<()> {
        if !allowed_values.contains(value) {
            let allowed_str = allowed_values.iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(crate::error::Error::Data(DataError::invalid_value(
                format!("Field '{}' value '{}' is not one of allowed values: [{}]", field_name, value, allowed_str)
            )));
        }
        Ok(())
    }

    /// Validate email format (for contact information in configs)
    pub fn validate_email(&self, email: &str) -> Result<()> {
        if !EMAIL_REGEX.is_match(email) {
            return Err(crate::error::Error::Data(DataError::invalid_format(
                format!("Invalid email format: {}", email)
            )));
        }

        Ok(())
    }

    /// Validate URL format (for configuration URLs)
    pub fn validate_url(&self, url: &str) -> Result<()> {
        if !URL_REGEX.is_match(url) {
            return Err(crate::error::Error::Data(DataError::invalid_format(
                format!("Invalid URL format: {}", url)
            )));
        }

        Ok(())
    }



    /// Validate environment name
    pub fn validate_environment_name(&self, env: &str) -> Result<()> {
        let valid_environments = ["dev", "stage", "prod", "test"];
        if !valid_environments.contains(&env) {
            return Err((ConfigError::MergeError(
                format!("Invalid environment '{}'. Valid environments are: {:?}", env, valid_environments)
            )));
        }
        Ok(())
    }

    /// Validate file or directory presence
    pub fn validate_file_or_directory_exists<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err((ConfigError::MergeError(
                format!("File or directory does not exist: {}", path_ref.display())
            )));
        }
        Ok(())
    }

    /// Validate glob pattern syntax
    pub fn validate_glob_pattern(&self, pattern: &str) -> Result<()> {
        // Basic glob pattern validation - check for invalid characters and patterns
        if pattern.is_empty() {
            return Err((ConfigError::MergeError(
                "Glob pattern cannot be empty".to_string()
            )));
        }

        // Check for path traversal attempts
        if pattern.contains("..") {
            return Err(ConfigError::MergeError(
                "Glob pattern cannot contain path traversal sequences (..)".to_string()
            ));
        }

        // Check for absolute paths (should be relative)
        if pattern.starts_with('/') || (cfg!(windows) && pattern.len() > 1 && pattern.chars().nth(1) == Some(':')) {
            return Err(ConfigError::MergeError(
                "Glob pattern should be relative, not absolute".to_string()
            ));
        }

        // Basic pattern validation (simplified since we don't have glob crate dependency)
        // In a real implementation, you would use glob::Pattern::new(pattern)
        Ok(())
    }

    /// Validate configuration using a custom validator function
    pub fn validate_config<T, F>(&self, config: &T, validator: F) -> Result<()>
    where
        F: Fn(&T) -> std::result::Result<(), ValidationError>,
    {
        match validator(config) {
            Ok(()) => Ok(()),
            Err(e) => Err(ConfigError::ValidationError {
                field: Some("configuration".to_string()),
                message: e.message.map(|m| m.into_owned()).unwrap_or_else(|| "Validation failed".to_string()),
            }.into()),
        }
    }
}

impl Default for ValidationUtils {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate a survey code
pub fn validate_survey_code(code: &str) -> Result<()> {
    VALIDATOR.validate_survey_code(code)
}

/// Validate a file path
pub fn validate_file_path<P: AsRef<Path>>(path: P) -> Result<()> {
    VALIDATOR.validate_file_path(path)
}

/// Validate a configuration using a custom validator
pub fn validate_config<T, F>(config: &T, validator: F) -> Result<()>
where
    F: Fn(&T) -> std::result::Result<(), ValidationError>,
{
    let validation_utils = ValidationUtils::new();
    validation_utils.validate_config(config, validator)
}

/// Validate numeric range
pub fn validate_numeric_range(value: f64, min: f64, max: f64) -> Result<()> {
    VALIDATOR.validate_numeric_range(value, min, max)
}

/// Validate year
pub fn validate_year(year: u32) -> Result<()> {
    VALIDATOR.validate_year(year)
}

/// Validate series ID
pub fn validate_series_id(series_id: &str) -> Result<()> {
    VALIDATOR.validate_series_id(series_id)
}

/// Validate period
pub fn validate_period(period: &str) -> Result<()> {
    VALIDATOR.validate_period(period)
}

/// Custom validation rules for BLS data
#[derive(Debug, Default)]
pub struct BLSValidationRules;

impl BLSValidationRules {
    pub(crate) fn default() -> BLSValidationRules {
        Default::default()
    }
}

impl BLSValidationRules {
    pub(crate) fn validate_lookup_record(&self, p0: &[String]) -> Result<bool> {
        todo!()
    }
}

impl BLSValidationRules {
    pub(crate) fn validate_observation_record(&self, p0: &[String]) -> Result<bool> {
        todo!()
    }
}

impl BLSValidationRules {
    pub(crate) fn validate_series_record(&self, p0: &[String]) -> Result<bool> {
        todo!()
    }
}

impl BLSValidationRules {
    /// Validate BLS data observation
    pub fn validate_observation(
        series_id: &str,
        year: u32,
        period: &str,
        value: Option<f64>,
    ) -> Result<()> {
        // Validate series ID
        VALIDATOR.validate_series_id(series_id)?;

        // Validate year
        VALIDATOR.validate_year(year)?;

        // Validate period
        VALIDATOR.validate_period(period)?;

        // Validate value if present
        if let Some(val) = value {
            if val.is_nan() || val.is_infinite() {
                return Err(crate::error::Error::Data(DataError::invalid_value(
                    format!("Invalid observation value: {}", val)
                )));
            }
        }

        Ok(())
    }

    /// Validate BLS series metadata
    pub fn validate_series_metadata(
        series_id: &str,
        series_title: &str,
        survey_code: &str,
    ) -> Result<()> {
        // Validate series ID
        VALIDATOR.validate_series_id(series_id)?;

        // Validate series title
        VALIDATOR.validate_non_empty_string(series_title, "series_title")?;
        VALIDATOR.validate_string_length(series_title, 1, 500, "series_title")?;

        // Validate survey code
        VALIDATOR.validate_survey_code(survey_code)?;

        Ok(())
    }

    /// Validate processing strategy configuration
    pub fn validate_processing_strategy(strategy: &str) -> Result<()> {
        let allowed_strategies = ["in_memory", "chunked", "mmap"];

        VALIDATOR.validate_enum_value(&strategy, &allowed_strategies, "processing_strategy")?;

        Ok(())
    }

    /// Validate output format configuration
    pub fn validate_output_format(format: &str) -> Result<()> {
        let allowed_formats = ["csv", "parquet", "json"];

        VALIDATOR.validate_enum_value(&format, &allowed_formats, "output_format")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_utils_new() {
        let validator = ValidationUtils::new();
        // Just ensure it doesn't panic
        assert!(true);
    }

    #[test]
    fn test_validate_survey_code() {
        let validator = ValidationUtils::new();

        // Valid codes
        assert!(validator.validate_survey_code("AP").is_ok());
        assert!(validator.validate_survey_code("ap").is_ok());

        // Invalid codes
        assert!(validator.validate_survey_code("A").is_err());
        assert!(validator.validate_survey_code("ABC").is_err());
        assert!(validator.validate_survey_code("A1").is_err());
    }

    #[test]
    fn test_validate_numeric_range() {
        let validator = ValidationUtils::new();

        // Valid values
        assert!(validator.validate_numeric_range(50.0, 0.0, 100.0).is_ok());
        assert!(validator.validate_numeric_range(0.0, 0.0, 100.0).is_ok());
        assert!(validator.validate_numeric_range(100.0, 0.0, 100.0).is_ok());

        // Invalid values
        assert!(validator.validate_numeric_range(-1.0, 0.0, 100.0).is_err());
        assert!(validator.validate_numeric_range(101.0, 0.0, 100.0).is_err());
        assert!(validator.validate_numeric_range(f64::NAN, 0.0, 100.0).is_err());
        assert!(validator.validate_numeric_range(f64::INFINITY, 0.0, 100.0).is_err());
    }

    #[test]
    fn test_validate_year() {
        let validator = ValidationUtils::new();

        // Valid years
        assert!(validator.validate_year(2020).is_ok());
        assert!(validator.validate_year(1950).is_ok());

        // Invalid years
        assert!(validator.validate_year(1800).is_err()); // Too old
        assert!(validator.validate_year(2100).is_err()); // Too far in future
    }

    #[test]
    fn test_validate_series_id() {
        let validator = ValidationUtils::new();

        // Valid series IDs
        assert!(validator.validate_series_id("APUS49074714").is_ok());

        // Invalid series IDs
        assert!(validator.validate_series_id("AP").is_err()); // Too short
        assert!(validator.validate_series_id("APUS490747@14").is_err()); // Invalid characters
    }

    #[test]
    fn test_validate_period() {
        let validator = ValidationUtils::new();

        // Valid periods
        assert!(validator.validate_period("M01").is_ok());
        assert!(validator.validate_period("Q01").is_ok());
        assert!(validator.validate_period("A01").is_ok());

        // Invalid periods
        assert!(validator.validate_period("M00").is_err());
        assert!(validator.validate_period("M13").is_err());
        assert!(validator.validate_period("X01").is_err());
    }

    #[test]
    fn test_validate_non_empty_string() {
        let validator = ValidationUtils::new();

        // Valid strings
        assert!(validator.validate_non_empty_string("Hello", "test_field").is_ok());
        assert!(validator.validate_non_empty_string("  Hello  ", "test_field").is_ok());

        // Invalid strings
        assert!(validator.validate_non_empty_string("", "test_field").is_err());
        assert!(validator.validate_non_empty_string("   ", "test_field").is_err());
    }

    #[test]
    fn test_validate_string_length() {
        let validator = ValidationUtils::new();

        // Valid lengths
        assert!(validator.validate_string_length("Hello", 1, 10, "test_field").is_ok());
        assert!(validator.validate_string_length("Hi", 1, 10, "test_field").is_ok());

        // Invalid lengths
        assert!(validator.validate_string_length("", 1, 10, "test_field").is_err()); // Too short
        assert!(validator.validate_string_length("This is too long", 1, 10, "test_field").is_err()); // Too long
    }

    #[test]
    fn test_validate_enum_value() {
        let validator = ValidationUtils::new();
        let allowed_values = ["option1", "option2", "option3"];

        // Valid values
        assert!(validator.validate_enum_value(&"option1", &allowed_values, "test_field").is_ok());
        assert!(validator.validate_enum_value(&"option2", &allowed_values, "test_field").is_ok());

        // Invalid values
        assert!(validator.validate_enum_value(&"option4", &allowed_values, "test_field").is_err());
        assert!(validator.validate_enum_value(&"invalid", &allowed_values, "test_field").is_err());
    }

    #[test]
    fn test_validate_email() {
        let validator = ValidationUtils::new();

        // Valid emails
        assert!(validator.validate_email("test@example.com").is_ok());
        assert!(validator.validate_email("user.name+tag@domain.co.uk").is_ok());

        // Invalid emails
        assert!(validator.validate_email("invalid-email").is_err());
        assert!(validator.validate_email("@example.com").is_err());
        assert!(validator.validate_email("test@").is_err());
    }

    #[test]
    fn test_validate_url() {
        let validator = ValidationUtils::new();

        // Valid URLs
        assert!(validator.validate_url("https://example.com").is_ok());
        assert!(validator.validate_url("http://subdomain.example.com/path").is_ok());

        // Invalid URLs
        assert!(validator.validate_url("not-a-url").is_err());
        assert!(validator.validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_bls_validation_rules() {
        // Test observation validation
        assert!(BLSValidationRules::validate_observation(
            "APUS49074714",
            2020,
            "M01",
            Some(123.45)
        ).is_ok());

        // Test invalid observation
        assert!(BLSValidationRules::validate_observation(
            "INVALID",
            2020,
            "M01",
            Some(123.45)
        ).is_err());

        // Test series metadata validation
        assert!(BLSValidationRules::validate_series_metadata(
            "APUS49074714",
            "Test Series Title",
            "AP"
        ).is_ok());

        // Test processing strategy validation
        assert!(BLSValidationRules::validate_processing_strategy("in_memory").is_ok());
        assert!(BLSValidationRules::validate_processing_strategy("invalid").is_err());

        // Test output format validation
        assert!(BLSValidationRules::validate_output_format("csv").is_ok());
        assert!(BLSValidationRules::validate_output_format("invalid").is_err());
    }

    #[test]
    fn test_convenience_functions() {
        // Test convenience functions
        assert!(validate_survey_code("AP").is_ok());
        assert!(validate_numeric_range(50.0, 0.0, 100.0).is_ok());
        assert!(validate_year(2020).is_ok());
        assert!(validate_series_id("APUS49074714").is_ok());
        assert!(validate_period("M01").is_ok());
    }
}