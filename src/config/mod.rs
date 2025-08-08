//! # Configuration Module
//!
//! This module provides configuration handling for the Rusty BLS Data Processing system.
//! It supports loading, validation, and management of survey-specific configurations.
//!
//! ## Components
//!
//! - [`model`]: Configuration data models and structures
//! - [`loader`]: Configuration loading from various sources (YAML, JSON, etc.)
//! - [`validator`]: Configuration validation and integrity checks
//!
//! ## Usage
//!
//! ```rust
//! use crate::rusty::{SurveyConfig, ConfigLoader};
//!
//! // Load configuration from file
//! let config = ConfigLoader::load_from_file("config/surveys/ap.yml")?;
//!
//! // Load configuration for a specific survey
//! let config = ConfigLoader::load_for_survey("ap")?;
//!
//! // Validate configuration
//! config.validate()?;
//! ```

// Module declarations
pub mod model;
pub mod loader;
pub mod validator;

// Re-export commonly used types and functions
pub use model::{
    Config, ConfigValue, SurveyConfig, OverviewConfig, ModelConfig, IoConfig, ProcessingConfig, 
    OutputConfig, DagsConfig, QualityConfig, RuntimeConfig, Overrides,
    FieldDefinition, ValidationRule, ProcessingStrategy, OutputFormat,
    CompressionAlgorithm, PartitioningStrategy, BackoffStrategy, QualityCheckType,
    QualitySeverity, LogLevel
};
pub use loader::{ConfigLoader, ConfigSource, ConfigFormat};
pub use validator::{ConfigValidator, ValidationResult, ValidationError, ValidationWarning};

// Convenience type aliases
pub type Result<T> = std::result::Result<T, crate::error::ConfigError>;

/// Load and validate a survey configuration with environment support
/// 
/// This is the main entry point for loading survey configurations.
/// It handles the complete load + merge + validate workflow.
/// 
/// # Arguments
/// * `survey_code` - The survey code (e.g., "AP", "BD")
/// * `env` - Optional environment ("dev", "stage", "prod"). Defaults to "dev"
/// 
/// # Returns
/// A fully loaded and validated `SurveyConfig`
/// 
/// # Example
/// ```rust
/// use rusty::config::load_survey_config;
/// 
/// // Load with default environment (dev)
/// let config = load_survey_config("AP", None)?;
/// 
/// // Load with specific environment
/// let config = load_survey_config("AP", Some("prod"))?;
/// ```
pub fn load_survey_config(survey_code: &str, env: Option<&str>) -> Result<SurveyConfig> {
    let loader = ConfigLoader::new();
    let environment = env.unwrap_or("dev");
    
    // Load the configuration
    let mut config = loader.load_survey_config(survey_code, Some(environment))
        .map_err(|e| crate::error::ConfigError::LoadError {
            path: format!("survey config for {}", survey_code),
            source: format!("{}", e),
        })?;
    
    // Validate the configuration
    let validator = ConfigValidator::new();
    let validation_result = validator.validate_survey_config(&config)
        .map_err(|e| crate::error::ConfigError::ValidationError {
            message: format!("Validation error: {}", e),
            field: None,
        })?;
    
    if !validation_result.is_valid() {
        return Err(crate::error::ConfigError::ValidationError {
            message: format!("Configuration validation failed for survey {}: {:?}", 
                           survey_code, validation_result.errors()),
            field: None,
        });
    }
    
    // Update metadata after successful load and validation
    config.update();
    
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all modules are accessible
        // This is a compile-time test to ensure all modules are properly exported
        let _loader = ConfigLoader::new();
        let _validator = ConfigValidator::new();
    }
}