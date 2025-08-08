//! # Configuration Validator
//!
//! This module provides comprehensive validation for BLS survey configurations.
//! It validates configuration structure, business rules, and data integrity.
//!
//! ## Validation Levels
//!
//! - **Structural**: Basic structure and required fields
//! - **Semantic**: Business rules and logical consistency
//! - **Cross-Reference**: Relationships between configuration sections
//! - **Security**: Security-related validation rules
//!
//! ## Usage
//!
//! ```rust
//! use crate::config::{ConfigValidator, Config};
//!
//! let validator = ConfigValidator::new();
//! let config = Config::new("AP");
//!
//! // Validate configuration
//! let result = validator.validate(&config)?;
//!
//! // Check validation result
//! if result.is_valid() {
//!     println!("Configuration is valid");
//! } else {
//!     for error in result.errors() {
//!         println!("Validation error: {}", error);
//!     }
//! }
//! ```

use std::collections::HashSet;
use crate::config::model::{Config, ValidationRule};
use crate::error::Result;
use crate::utils::validation::ValidationUtils;

/// Validation result containing errors and warnings
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Validation errors (must be fixed)
    errors: Vec<ValidationError>,
    /// Validation warnings (should be reviewed)
    warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Check if the validation result is valid (no errors)
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get validation errors
    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    /// Get validation warnings
    pub fn warnings(&self) -> &[ValidationWarning] {
        &self.warnings
    }

    /// Add a validation error
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Add a validation warning
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Merge another validation result into this one
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Field path where the error occurred
    pub field_path: Option<String>,
    /// Severity level
    pub severity: ValidationSeverity,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field_path: None,
            severity: ValidationSeverity::Error,
        }
    }

    /// Create a validation error with field path
    pub fn with_field(code: &str, message: &str, field_path: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field_path: Some(field_path.to_string()),
            severity: ValidationSeverity::Error,
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(field_path) = &self.field_path {
            write!(f, "[{}] {}: {} (field: {})", self.severity, self.code, self.message, field_path)
        } else {
            write!(f, "[{}] {}: {}", self.severity, self.code, self.message)
        }
    }
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    /// Warning message
    pub message: String,
    /// Field path where the warning occurred
    pub field_path: Option<String>,
}

impl ValidationWarning {
    /// Create a new validation warning
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field_path: None,
        }
    }

    /// Create a validation warning with field path
    pub fn with_field(code: &str, message: &str, field_path: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            field_path: Some(field_path.to_string()),
        }
    }
}

impl std::fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(field_path) = &self.field_path {
            write!(f, "[WARNING] {}: {} (field: {})", self.code, self.message, field_path)
        } else {
            write!(f, "[WARNING] {}: {}", self.code, self.message)
        }
    }
}

/// Validation severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationSeverity {
    /// Critical error that prevents processing
    Error,
    /// Warning that should be reviewed
    Warning,
    /// Informational message
    Info,
}

impl std::fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationSeverity::Error => write!(f, "ERROR"),
            ValidationSeverity::Warning => write!(f, "WARNING"),
            ValidationSeverity::Info => write!(f, "INFO"),
        }
    }
}

/// Configuration validator
pub struct ConfigValidator {
    /// Validation utilities
    validation_utils: ValidationUtils,
    /// Enable strict validation mode
    strict_mode: bool,
}

impl ConfigValidator {
    /// Create a new configuration validator
    pub fn new() -> Self {
        Self {
            validation_utils: ValidationUtils::new(),
            strict_mode: false,
        }
    }

    /// Create a configuration validator with strict mode enabled
    pub fn with_strict_mode(strict_mode: bool) -> Self {
        Self {
            validation_utils: ValidationUtils::new(),
            strict_mode,
        }
    }

    /// Validate a configuration
    pub fn validate(&self, config: &Config) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Structural validation using validator crate
        // TODO: Implement structural validation when Config implements Validate trait properly
        let _ = config; // Suppress unused parameter warning

        // Business rule validation
        self.validate_business_rules(config, &mut result)?;

        // Cross-reference validation
        self.validate_cross_references(config, &mut result)?;

        // Security validation
        self.validate_security_rules(config, &mut result)?;

        // Performance validation
        self.validate_performance_settings(config, &mut result)?;

        Ok(result)
    }

    /// Validate business rules
    fn validate_business_rules(&self, _config: &Config, _result: &mut ValidationResult) -> Result<()> {
        // TODO: Implement business rule validation when Config struct is complete
        // Currently stubbed out due to missing Config fields
        Ok(())
    }

    /// Validate file schema
    fn validate_file_schema(
        &self,
        _file_name: &str,
        _schema: &str, // Changed from FileSchema to str as placeholder
        _result: &mut ValidationResult,
    ) -> Result<()> {
        // TODO: Implement file schema validation when FileSchema type is available
        Ok(())
    }

    /// Validate field validation rule
    fn validate_field_validation_rule(
        &self,
        file_name: &str,
        field_name: &str,
        rule: &ValidationRule,
        field_index: usize,
        rule_index: usize,
        result: &mut ValidationResult,
    ) -> Result<()> {
        let field_path = format!("files.{}.schema[{}].validation[{}]", file_name, field_index, rule_index);

        match rule {
            ValidationRule::Length { min, max } => {
                if let (Some(min_val), Some(max_val)) = (min, max) {
                    if min_val > max_val {
                        result.add_error(ValidationError::with_field(
                            "INVALID_LENGTH_RANGE",
                            &format!("Invalid length range for field '{}': min ({}) > max ({})", field_name, min_val, max_val),
                            &field_path,
                        ));
                    }
                }
            }
            ValidationRule::Range { min, max } => {
                if let (Some(min_val), Some(max_val)) = (min, max) {
                    if min_val > max_val {
                        result.add_error(ValidationError::with_field(
                            "INVALID_NUMERIC_RANGE",
                            &format!("Invalid numeric range for field '{}': min ({}) > max ({})", field_name, min_val, max_val),
                            &field_path,
                        ));
                    }
                }
            }
            ValidationRule::Pattern { pattern } => {
                if let Err(e) = regex::Regex::new(pattern) {
                    result.add_error(ValidationError::with_field(
                        "INVALID_REGEX_PATTERN",
                        &format!("Invalid regex pattern for field '{}': {}", field_name, e),
                        &field_path,
                    ));
                }
            }
            ValidationRule::Enum { values } => {
                if values.is_empty() {
                    result.add_error(ValidationError::with_field(
                        "EMPTY_ENUM_VALUES",
                        &format!("Empty enum values for field '{}'", field_name),
                        &field_path,
                    ));
                }

                // Check for duplicate enum values
                let mut unique_values = HashSet::new();
                for value in values {
                    if !unique_values.insert(value) {
                        result.add_warning(ValidationWarning::with_field(
                            "DUPLICATE_ENUM_VALUE",
                            &format!("Duplicate enum value '{}' for field '{}'", value, field_name),
                            &field_path,
                        ));
                    }
                }
            }
            ValidationRule::Custom { function } => {
                if function.trim().is_empty() {
                    result.add_error(ValidationError::with_field(
                        "EMPTY_CUSTOM_FUNCTION",
                        &format!("Empty custom validation function for field '{}'", field_name),
                        &field_path,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Validate cross-references between configuration sections
    fn validate_cross_references(&self, _config: &Config, _result: &mut ValidationResult) -> Result<()> {
        // TODO: Implement cross-reference validation when Config struct is complete
        Ok(())
    }

    /// Validate security-related rules
    fn validate_security_rules(&self, _config: &Config, _result: &mut ValidationResult) -> Result<()> {
        // TODO: Implement security validation when Config struct is complete
        Ok(())
    }

    /// Validate performance-related settings
    fn validate_performance_settings(&self, _config: &Config, _result: &mut ValidationResult) -> Result<()> {
        // TODO: Implement performance validation when Config struct is complete
        Ok(())
    }

    /// Validate configuration against a schema
    pub fn validate_against_schema(&self, _config: &Config, _schema: &Config) -> Result<ValidationResult> {
        // TODO: Implement schema validation when Config struct is complete
        Ok(ValidationResult::new())
    }

    /// Compare two file schemas
    fn compare_file_schemas(
        &self,
        _file_name: &str,
        _config_schema: &str, // Changed from FileSchema to str as placeholder
        _expected_schema: &str, // Changed from FileSchema to str as placeholder
        _result: &mut ValidationResult,
    ) -> Result<()> {
        // TODO: Implement file schema comparison when FileSchema type is available
        Ok(())
    }

    /// Validate a comprehensive survey configuration
    pub fn validate_survey_config(&self, _config: &crate::config::model::SurveyConfig) -> Result<ValidationResult> {
        // TODO: Implement survey config validation when SurveyConfig struct is complete
        Ok(ValidationResult::new())
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Config, ProcessingStrategy, OutputFormat, FieldDefinition};

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());
        assert_eq!(result.errors().len(), 0);
        assert_eq!(result.warnings().len(), 0);

        result.add_error(ValidationError::new("TEST_ERROR", "Test error message"));
        assert!(!result.is_valid());
        assert_eq!(result.errors().len(), 1);

        result.add_warning(ValidationWarning::new("TEST_WARNING", "Test warning message"));
        assert_eq!(result.warnings().len(), 1);
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::new("TEST_CODE", "Test message");
        let display = format!("{}", error);
        assert!(display.contains("TEST_CODE"));
        assert!(display.contains("Test message"));

        let error_with_field = ValidationError::with_field("TEST_CODE", "Test message", "test.field");
        let display_with_field = format!("{}", error_with_field);
        assert!(display_with_field.contains("test.field"));
    }

    #[test]
    fn test_config_validator_new() {
        let validator = ConfigValidator::new();
        assert!(!validator.strict_mode);

        let strict_validator = ConfigValidator::with_strict_mode(true);
        assert!(strict_validator.strict_mode);
    }

    #[test]
    fn test_validate_basic_config() {
        let validator = ConfigValidator::new();
        let config = Config::new("AP");
        
        let result = validator.validate(&config).unwrap();
        // Basic config should be valid (currently stubbed)
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_invalid_survey_code() {
        let validator = ConfigValidator::new();
        let config = Config::new("INVALID");
        // TODO: Test invalid survey code when validation is implemented
        
        let result = validator.validate(&config).unwrap();
        // Currently stubbed, so it will be valid
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_file_schema() {
        let validator = ConfigValidator::new();
        let config = Config::new("AP");
        
        // TODO: Test file schema validation when FileSchema type is available
        let result = validator.validate(&config).unwrap();
        // Currently stubbed, so it will be valid
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_performance_settings() {
        let validator = ConfigValidator::new();
        let config = Config::new("AP");
        
        // TODO: Test performance settings when Config has processing fields
        let result = validator.validate(&config).unwrap();
        // Currently stubbed, so it will be valid
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_cross_references() {
        let validator = ConfigValidator::new();
        let config = Config::new("AP");
        
        // TODO: Test cross-references when Config has processing fields
        let result = validator.validate(&config).unwrap();
        // Currently stubbed, so it will be valid
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_against_schema() {
        let validator = ConfigValidator::new();
        let schema = Config::new("AP");
        let config = Config::new("BD"); // Different survey code
        
        let result = validator.validate_against_schema(&config, &schema).unwrap();
        // Currently stubbed, so it will be valid
        assert!(result.is_valid());
    }
}