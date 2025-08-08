//! # Output Factory Implementation
//!
//! This module provides factory implementations for creating and managing
//! output generators and format writers.

use std::collections::HashMap;

use crate::output::traits::{OutputFactory, OutputGenerator, FormatWriter, OutputConfig};
use crate::output::format::{create_format_generator, create_format_writer, supported_formats};
use crate::error::types::{ProcessingError, Result};

/// Default implementation of the output factory
pub struct DefaultOutputFactory {
    /// Factory configuration
    config: FactoryConfig,
    /// Cached default configurations for formats
    default_configs: HashMap<String, OutputConfig>,
}

/// Configuration for the output factory
#[derive(Debug, Clone)]
pub struct FactoryConfig {
    /// Enable caching of generators
    pub enable_caching: bool,
    /// Maximum cache size
    pub max_cache_size: usize,
    /// Default output directory
    pub default_output_dir: String,
}

impl Default for FactoryConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            max_cache_size: 100,
            default_output_dir: "data/processed".to_string(),
        }
    }
}

impl DefaultOutputFactory {
    /// Create a new output factory
    pub fn new() -> Self {
        let mut factory = Self {
            config: FactoryConfig::default(),
            default_configs: HashMap::new(),
        };
        
        // Initialize default configurations for supported formats
        factory.initialize_default_configs();
        factory
    }

    /// Create a new output factory with custom configuration
    pub fn with_config(config: FactoryConfig) -> Self {
        let mut factory = Self {
            config,
            default_configs: HashMap::new(),
        };
        
        factory.initialize_default_configs();
        factory
    }

    /// Get factory configuration
    pub fn config(&self) -> &FactoryConfig {
        &self.config
    }

    /// Initialize default configurations for all supported formats
    fn initialize_default_configs(&mut self) {
        // CSV default configuration
        let csv_config = OutputConfig {
            format: "csv".to_string(),
            destination: format!("{}/output.csv", self.config.default_output_dir),
            compression: None,
            partitioning: None,
            format_options: {
                let mut options = HashMap::new();
                options.insert("delimiter".to_string(), ",".to_string());
                options.insert("quote_char".to_string(), "\"".to_string());
                options.insert("header".to_string(), "true".to_string());
                options
            },
            metadata: HashMap::new(),
        };
        self.default_configs.insert("csv".to_string(), csv_config);

        // JSON default configuration
        let json_config = OutputConfig {
            format: "json".to_string(),
            destination: format!("{}/output.json", self.config.default_output_dir),
            compression: None,
            partitioning: None,
            format_options: {
                let mut options = HashMap::new();
                options.insert("pretty".to_string(), "true".to_string());
                options.insert("indent".to_string(), "2".to_string());
                options
            },
            metadata: HashMap::new(),
        };
        self.default_configs.insert("json".to_string(), json_config);

        // Parquet default configuration
        let parquet_config = OutputConfig {
            format: "parquet".to_string(),
            destination: format!("{}/output.parquet", self.config.default_output_dir),
            compression: Some(crate::output::traits::CompressionConfig {
                algorithm: "snappy".to_string(),
                level: None,
                enabled: true,
            }),
            partitioning: None,
            format_options: {
                let mut options = HashMap::new();
                options.insert("row_group_size".to_string(), "100000".to_string());
                options.insert("enable_dictionary".to_string(), "true".to_string());
                options
            },
            metadata: HashMap::new(),
        };
        self.default_configs.insert("parquet".to_string(), parquet_config);
    }

    /// Validate format and configuration
    fn validate_format_and_config(&self, format: &str, config: &OutputConfig) -> Result<()> {
        // Check if format is supported
        if !self.supports_format(format) {
            return Err(ProcessingError::resource_exhausted(
                format!("Unsupported format: {}", format)
            ));
        }

        // Check if config format matches requested format
        if config.format.to_lowercase() != format.to_lowercase() {
            return Err(ProcessingError::resource_exhausted(
                format!("Configuration format '{}' does not match requested format '{}'", 
                       config.format, format)
            ));
        }

        // Validate destination path
        if config.destination.is_empty() {
            return Err(ProcessingError::resource_exhausted(
                "Output destination cannot be empty".to_string()
            ));
        }

        Ok(())
    }
}

impl Default for DefaultOutputFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFactory for DefaultOutputFactory {
    fn create_generator(&self, format: &str, config: OutputConfig) -> Result<Box<dyn OutputGenerator>> {
        // Validate format and configuration
        self.validate_format_and_config(format, &config)?;

        // Create the generator using the format module
        let mut generator = create_format_generator(format)?;

        // Validate the configuration with the generator
        generator.validate_config(&config)?;

        Ok(generator)
    }

    fn create_writer(&self, format: &str) -> Result<Box<dyn FormatWriter>> {
        // Check if format is supported
        if !self.supports_format(format) {
            return Err(ProcessingError::resource_exhausted(
                format!("Unsupported format: {}", format)
            ));
        }

        // Create the writer using the format module
        create_format_writer(format)
    }

    fn supported_formats(&self) -> Vec<String> {
        supported_formats()
    }

    fn supports_format(&self, format: &str) -> bool {
        self.supported_formats().contains(&format.to_lowercase())
    }

    fn default_config_for_format(&self, format: &str) -> Result<OutputConfig> {
        let format_lower = format.to_lowercase();
        
        if let Some(config) = self.default_configs.get(&format_lower) {
            Ok(config.clone())
        } else {
            Err(ProcessingError::resource_exhausted(
                format!("No default configuration available for format: {}", format)
            ))
        }
    }
}

/// Enhanced factory implementation with additional features
pub struct OutputFactoryImpl {
    /// Base factory
    base_factory: DefaultOutputFactory,
    /// Custom generator creators
    custom_generators: HashMap<String, Box<dyn Fn() -> Box<dyn OutputGenerator> + Send + Sync>>,
    /// Custom writer creators
    custom_writers: HashMap<String, Box<dyn Fn() -> Box<dyn FormatWriter> + Send + Sync>>,
}

impl OutputFactoryImpl {
    /// Create a new enhanced output factory
    pub fn new() -> Self {
        Self {
            base_factory: DefaultOutputFactory::new(),
            custom_generators: HashMap::new(),
            custom_writers: HashMap::new(),
        }
    }

    /// Register a custom generator creator
    pub fn register_generator_creator<F>(&mut self, format: String, creator: F)
    where
        F: Fn() -> Box<dyn OutputGenerator> + Send + Sync + 'static,
    {
        self.custom_generators.insert(format, Box::new(creator));
    }

    /// Register a custom writer creator
    pub fn register_writer_creator<F>(&mut self, format: String, creator: F)
    where
        F: Fn() -> Box<dyn FormatWriter> + Send + Sync + 'static,
    {
        self.custom_writers.insert(format, Box::new(creator));
    }

    /// Unregister a custom generator creator
    pub fn unregister_generator_creator(&mut self, format: &str) -> bool {
        self.custom_generators.remove(format).is_some()
    }

    /// Unregister a custom writer creator
    pub fn unregister_writer_creator(&mut self, format: &str) -> bool {
        self.custom_writers.remove(format).is_some()
    }

    /// List custom formats
    pub fn list_custom_formats(&self) -> Vec<String> {
        let mut formats = Vec::new();
        formats.extend(self.custom_generators.keys().cloned());
        formats.extend(self.custom_writers.keys().cloned());
        formats.sort();
        formats.dedup();
        formats
    }
}

impl Default for OutputFactoryImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFactory for OutputFactoryImpl {
    fn create_generator(&self, format: &str, config: OutputConfig) -> Result<Box<dyn OutputGenerator>> {
        let format_lower = format.to_lowercase();
        
        // Check for custom generator first
        if let Some(creator) = self.custom_generators.get(&format_lower) {
            let mut generator = creator();
            generator.validate_config(&config)?;
            return Ok(generator);
        }

        // Fall back to base factory
        self.base_factory.create_generator(format, config)
    }

    fn create_writer(&self, format: &str) -> Result<Box<dyn FormatWriter>> {
        let format_lower = format.to_lowercase();
        
        // Check for custom writer first
        if let Some(creator) = self.custom_writers.get(&format_lower) {
            return Ok(creator());
        }

        // Fall back to base factory
        self.base_factory.create_writer(format)
    }

    fn supported_formats(&self) -> Vec<String> {
        let mut formats = self.base_factory.supported_formats();
        formats.extend(self.list_custom_formats());
        formats.sort();
        formats.dedup();
        formats
    }

    fn supports_format(&self, format: &str) -> bool {
        let format_lower = format.to_lowercase();
        self.custom_generators.contains_key(&format_lower) ||
        self.custom_writers.contains_key(&format_lower) ||
        self.base_factory.supports_format(format)
    }

    fn default_config_for_format(&self, format: &str) -> Result<OutputConfig> {
        // For custom formats, create a basic default config
        let format_lower = format.to_lowercase();
        if self.custom_generators.contains_key(&format_lower) || 
           self.custom_writers.contains_key(&format_lower) {
            return Ok(OutputConfig {
                format: format_lower,
                destination: format!("data/processed/output.{}", format),
                ..Default::default()
            });
        }

        // Fall back to base factory
        self.base_factory.default_config_for_format(format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_factory_creation() {
        let factory = DefaultOutputFactory::new();
        assert!(!factory.supported_formats().is_empty());
        assert!(factory.supports_format("csv"));
        assert!(factory.supports_format("json"));
        assert!(factory.supports_format("parquet"));
    }

    #[test]
    fn test_factory_with_config() {
        let config = FactoryConfig {
            enable_caching: false,
            max_cache_size: 50,
            default_output_dir: "/tmp/output".to_string(),
        };
        
        let factory = DefaultOutputFactory::with_config(config);
        assert!(!factory.config().enable_caching);
        assert_eq!(factory.config().max_cache_size, 50);
        assert_eq!(factory.config().default_output_dir, "/tmp/output");
    }

    #[test]
    fn test_create_csv_generator() {
        let factory = DefaultOutputFactory::new();
        let config = factory.default_config_for_format("csv").unwrap();
        
        let generator = factory.create_generator("csv", config);
        assert!(generator.is_ok());
        
        let generator = generator.unwrap();
        assert_eq!(generator.name(), "csv_generator");
    }

    #[test]
    fn test_create_csv_writer() {
        let factory = DefaultOutputFactory::new();
        let writer = factory.create_writer("csv");
        assert!(writer.is_ok());
        
        let writer = writer.unwrap();
        assert_eq!(writer.format_name(), "csv");
    }

    #[test]
    fn test_create_unsupported_generator() {
        let factory = DefaultOutputFactory::new();
        let config = OutputConfig::default();
        
        let result = factory.create_generator("unknown", config);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_config_for_formats() {
        let factory = DefaultOutputFactory::new();
        
        let csv_config = factory.default_config_for_format("csv");
        assert!(csv_config.is_ok());
        assert_eq!(csv_config.unwrap().format, "csv");
        
        let json_config = factory.default_config_for_format("json");
        assert!(json_config.is_ok());
        assert_eq!(json_config.unwrap().format, "json");
        
        let parquet_config = factory.default_config_for_format("parquet");
        assert!(parquet_config.is_ok());
        assert_eq!(parquet_config.unwrap().format, "parquet");
    }

    #[test]
    fn test_validate_format_and_config() {
        let factory = DefaultOutputFactory::new();
        
        let valid_config = OutputConfig {
            format: "csv".to_string(),
            destination: "output.csv".to_string(),
            ..Default::default()
        };
        assert!(factory.validate_format_and_config("csv", &valid_config).is_ok());
        
        let invalid_format_config = OutputConfig {
            format: "json".to_string(),
            destination: "output.json".to_string(),
            ..Default::default()
        };
        assert!(factory.validate_format_and_config("csv", &invalid_format_config).is_err());
        
        let empty_destination_config = OutputConfig {
            format: "csv".to_string(),
            destination: "".to_string(),
            ..Default::default()
        };
        assert!(factory.validate_format_and_config("csv", &empty_destination_config).is_err());
    }

    #[test]
    fn test_enhanced_factory() {
        let mut factory = OutputFactoryImpl::new();
        
        // Should support base formats
        assert!(factory.supports_format("csv"));
        assert!(factory.supports_format("json"));
        
        // Register a custom generator
        factory.register_generator_creator("custom".to_string(), || {
            Box::new(crate::output::format::CsvOutputGenerator::new())
        });
        
        assert!(factory.supports_format("custom"));
        assert!(factory.list_custom_formats().contains(&"custom".to_string()));
        
        // Unregister custom generator
        assert!(factory.unregister_generator_creator("custom"));
        assert!(!factory.supports_format("custom"));
    }

    #[test]
    fn test_enhanced_factory_custom_config() {
        let factory = OutputFactoryImpl::new();
        
        // Test default config for non-existent custom format
        let config = factory.default_config_for_format("unknown");
        assert!(config.is_err());
    }
}