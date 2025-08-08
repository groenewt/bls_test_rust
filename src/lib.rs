//! # Rusty BLS Data Processing Library
//!
//! A comprehensive, enterprise-level data processing system for Bureau of Labor Statistics (BLS) survey data.
//! 
//! ## Overview
//! 
//! Rusty is designed to:
//! - Read raw BLS data files in various formats
//! - Apply transformations and validations
//! - Output processed data in more usable formats (CSV, Parquet, JSON)
//! 
//! ## Architecture
//! 
//! The system follows a modular, interface-based architecture with the following core principles:
//! 
//! 1. **Interface-Based Design**: Components interact through well-defined trait interfaces
//! 2. **Modular Structure**: Codebase is organized into logical modules with clear responsibilities
//! 3. **Plugin Architecture**: Support for dynamically loading survey-specific components
//! 4. **Configuration-Driven**: Behavior controlled by configuration rather than hardcoded logic
//! 5. **Performance Optimization**: Different strategies for different survey sizes
//! 
//! ## Module Structure
//! 
//! - [`config`]: Configuration handling and validation
//! - [`data`]: Data structures, models, and I/O operations
//! - [`processing`]: Data processing strategies and pipeline
//! - [`output`]: Output generation and formatting
//! - [`error`]: Comprehensive error handling system
//! - [`utils`]: Utility functions and helpers
//! - [`plugin`]: Plugin system for extensibility
//! 
//! ## Usage
//!
//! ```

// Core modules
pub mod config;
pub mod data;
pub mod processing;
pub mod output;
pub mod error;
pub mod utils;
pub mod plugin;

// Re-export commonly used types and traits
pub use error::{Error, Result};
pub use config::{
    SurveyConfig, ConfigLoader, load_survey_config,
    OverviewConfig, ModelConfig, IoConfig, ProcessingConfig, 
    OutputConfig, DagsConfig, QualityConfig, RuntimeConfig,
    ProcessingStrategy, OutputFormat
};
pub use data::{Series, Observation, Survey};
pub use processing::{ProcessingEngine};
pub use output::{OutputGenerator};

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Initialize the library with default logging configuration
pub fn init() -> Result<()> {
    env_logger::init();
    log::info!("Initialized {} v{}", NAME, VERSION);
    Ok(())
}

/// Initialize the library with tracing support for enterprise monitoring
pub fn init_with_tracing() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("Initialized {} v{} with tracing", NAME, VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "rusty");
        assert!(!DESCRIPTION.is_empty());
    }

    #[test]
    fn test_init() {
        // Test that init doesn't panic
        let result = init();
        assert!(result.is_ok());
    }
}