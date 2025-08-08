//! # Output Module
//!
//! This module provides comprehensive output generation capabilities for BLS survey data.
//! It supports multiple output formats and provides a flexible, extensible architecture
//! for adding new output formats and destinations.
//!
//! ## Architecture
//!
//! The output system is built around several key concepts:
//!
//! - **Output Generators**: High-level components that orchestrate output generation
//! - **Format Writers**: Low-level components that handle format-specific writing
//! - **Output Registry**: Dynamic registration and discovery of output generators
//! - **Output Factory**: Creation and configuration of output components
//!
//! ## Supported Formats
//!
//! - **CSV**: Comma-separated values format for easy data exchange
//! - **JSON**: JavaScript Object Notation for web applications
//! - **Parquet**: Columnar storage format for analytics (future enhancement)
//!
//! ## Usage
//!
//! ```rust
//! use rusty::output::{OutputConfig, OutputFactory, create_factory};
//! use rusty::processing::traits::ProcessedData;
//! use rusty::error::Result;
//!
//! async fn generate_output(data: ProcessedData) -> Result<()> {
//!     let factory = create_factory();
//!     let config = OutputConfig {
//!         format: "csv".to_string(),
//!         destination: "output.csv".to_string(),
//!         ..Default::default()
//!     };
//!     
//!     let mut generator = factory.create_generator("csv", config.clone())?;
//!     let result = generator.generate(data, config).await?;
//!     
//!     println!("Generated {} files with {} records",
//!              result.output_paths.len(), result.records_written);
//!     Ok(())
//! }
//! ```

pub mod factory;
pub mod format;
pub mod registry;
pub mod traits;

// Re-export commonly used types and traits
pub use traits::{
    CompressionConfig, FormatWriter, OutputConfig, OutputFactory, OutputGenerator, OutputRegistry,
    OutputResult, OutputStats, PartitionNamingStrategy, PartitioningConfig,
};

pub use format::{
    CsvOutputGenerator, CsvWriter, JsonOutputGenerator, JsonWriter, ParquetOutputGenerator,
    ParquetWriter,
};

pub use registry::{DefaultOutputRegistry, OutputRegistryImpl};

pub use factory::{DefaultOutputFactory, OutputFactoryImpl};

/// Create a default output factory with all standard formats
pub fn create_factory() -> DefaultOutputFactory {
    DefaultOutputFactory::new()
}

/// Create an output generator for the specified format
pub fn create_generator(
    format: &str,
    config: OutputConfig,
) -> crate::error::Result<Box<dyn OutputGenerator>> {
    let factory = create_factory();
    factory.create_generator(format, config)
}

/// Create a format writer for the specified format
pub fn create_writer(format: &str) -> crate::error::Result<Box<dyn FormatWriter>> {
    let factory = create_factory();
    factory.create_writer(format)
}

/// Get list of supported output formats
pub fn supported_formats() -> Vec<String> {
    let factory = create_factory();
    factory.supported_formats()
}

/// Check if a format is supported
pub fn is_format_supported(format: &str) -> bool {
    let factory = create_factory();
    factory.supports_format(format)
}

/// Get default configuration for a format
pub fn default_config_for_format(format: &str) -> crate::error::Result<OutputConfig> {
    let factory = create_factory();
    factory.default_config_for_format(format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_factory() {
        let factory = create_factory();
        assert!(!factory.supported_formats().is_empty());
    }

    #[test]
    fn test_supported_formats() {
        let formats = supported_formats();
        assert!(formats.contains(&"csv".to_string()));
    }

    #[test]
    fn test_is_format_supported() {
        assert!(is_format_supported("csv"));
        assert!(!is_format_supported("unknown"));
    }

    #[test]
    fn test_default_config_for_csv() {
        let config = default_config_for_format("csv");
        assert!(config.is_ok());

        let config = config.unwrap();
        assert_eq!(config.format, "csv");
    }

    #[test]
    fn test_create_csv_writer() {
        let writer = create_writer("csv");
        assert!(writer.is_ok());

        let writer = writer.unwrap();
        assert_eq!(writer.format_name(), "csv");
    }

    #[test]
    fn test_create_csv_generator() {
        let config = OutputConfig {
            format: "csv".to_string(),
            destination: "test.csv".to_string(),
            ..Default::default()
        };

        let generator = create_generator("csv", config);
        assert!(generator.is_ok());

        let generator = generator.unwrap();
        assert_eq!(generator.name(), "csv_generator");
    }
}
