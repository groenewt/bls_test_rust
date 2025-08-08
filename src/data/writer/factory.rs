//! Writer factory implementation for BLS data processing
//!
//! This module provides a factory for creating appropriate writer instances
//! based on output format, configuration, and performance requirements.
//! The factory automatically selects the optimal writer strategy.

use std::collections::HashMap;
use std::path::Path;

use crate::data::writer::csv_writer::CsvDataWriter;
use crate::data::writer::json_writer::JsonDataWriter;
use crate::data::writer::parquet_writer::ParquetDataWriter;
use crate::data::writer::traits::{
    CompressedWriter, DataWriter, LookupWriter, ObservationWriter, SeriesWriter, StreamingWriter,
    SurveyWriter, TransactionalWriter, WriterConfig, WriterFactory,
};
use crate::error::types::{DataError, Result};

/// Default writer factory implementation
pub struct DefaultWriterFactory {
    /// Configuration for writer selection
    config: WriterFactoryConfig,
    /// Cache of writer configurations by format
    format_configs: HashMap<String, WriterConfig>,
}

/// Configuration for the writer factory
#[derive(Debug, Clone)]
pub struct WriterFactoryConfig {
    /// Default buffer size for writers (in bytes)
    pub default_buffer_size: usize,
    /// Default batch size for writing operations
    pub default_batch_size: usize,
    /// Whether to enable validation by default
    pub enable_validation: bool,
    /// Whether to enable compression by default
    pub enable_compression: bool,
    /// Default compression level
    pub default_compression_level: u8,
    /// Maximum number of errors to tolerate by default
    pub max_errors: usize,
    /// Whether to include headers by default
    pub include_headers: bool,
    /// Default float precision
    pub float_precision: usize,
    /// Format-specific configurations
    pub format_configs: HashMap<String, WriterConfig>,
}

impl Default for WriterFactoryConfig {
    fn default() -> Self {
        Self {
            default_buffer_size: 64 * 1024, // 64KB
            default_batch_size: 1000,
            enable_validation: true,
            enable_compression: false,
            default_compression_level: 6,
            max_errors: 100,
            include_headers: true,
            float_precision: 6,
            format_configs: HashMap::new(),
        }
    }
}

impl Default for DefaultWriterFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultWriterFactory {
    /// Create a new writer factory with default configuration
    pub fn new() -> Self {
        Self::with_config(WriterFactoryConfig::default())
    }

    /// Create a new writer factory with the given configuration
    pub fn with_config(config: WriterFactoryConfig) -> Self {
        let mut format_configs = HashMap::new();

        // Set up default configurations for different formats
        format_configs.insert(
            "csv".to_string(),
            WriterConfig {
                buffer_size: config.default_buffer_size,
                batch_size: config.default_batch_size,
                validate_on_write: config.enable_validation,
                encoding: "UTF-8".to_string(),
                field_separator: ',',
                include_headers: config.include_headers,
                compress_output: config.enable_compression,
                compression_level: config.default_compression_level,
                overwrite_existing: false,
                max_file_size: 0, // No limit
                float_precision: config.float_precision,
            },
        );

        format_configs.insert(
            "tsv".to_string(),
            WriterConfig {
                buffer_size: config.default_buffer_size,
                batch_size: config.default_batch_size,
                validate_on_write: config.enable_validation,
                encoding: "UTF-8".to_string(),
                field_separator: '\t',
                include_headers: config.include_headers,
                compress_output: config.enable_compression,
                compression_level: config.default_compression_level,
                overwrite_existing: false,
                max_file_size: 0,
                float_precision: config.float_precision,
            },
        );

        format_configs.insert(
            "parquet".to_string(),
            WriterConfig {
                buffer_size: config.default_buffer_size,
                batch_size: config.default_batch_size * 10, // Larger batches for Parquet
                validate_on_write: config.enable_validation,
                encoding: "UTF-8".to_string(),
                field_separator: ',',   // Not used for Parquet
                include_headers: false, // Parquet has schema
                compress_output: true,  // Parquet benefits from compression
                compression_level: config.default_compression_level,
                overwrite_existing: false,
                max_file_size: 0,
                float_precision: config.float_precision,
            },
        );

        format_configs.insert(
            "json".to_string(),
            WriterConfig {
                buffer_size: config.default_buffer_size,
                batch_size: config.default_batch_size,
                validate_on_write: config.enable_validation,
                encoding: "UTF-8".to_string(),
                field_separator: ',',  // Not used for JSON
                include_headers: true, // Pretty printing
                compress_output: config.enable_compression,
                compression_level: config.default_compression_level,
                overwrite_existing: false,
                max_file_size: 0,
                float_precision: config.float_precision,
            },
        );

        format_configs.insert(
            "jsonl".to_string(),
            WriterConfig {
                buffer_size: config.default_buffer_size,
                batch_size: config.default_batch_size,
                validate_on_write: config.enable_validation,
                encoding: "UTF-8".to_string(),
                field_separator: ',',   // Not used for JSONL
                include_headers: false, // Compact format for JSONL
                compress_output: config.enable_compression,
                compression_level: config.default_compression_level,
                overwrite_existing: false,
                max_file_size: 0,
                float_precision: config.float_precision,
            },
        );

        // Override with user-provided configurations
        for (format, user_config) in &config.format_configs {
            format_configs.insert(format.clone(), user_config.clone());
        }

        Self {
            config,
            format_configs,
        }
    }

    /// Determine the output format from the file path
    fn determine_format(&self, path: &Path) -> Result<String> {
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension {
                "csv" => Ok("csv".to_string()),
                "tsv" => Ok("tsv".to_string()),
                "txt" => {
                    // Try to infer from filename or default to CSV
                    if let Some(filename) = path.file_stem().and_then(|n| n.to_str()) {
                        if filename.contains("tsv") {
                            Ok("tsv".to_string())
                        } else {
                            Ok("csv".to_string())
                        }
                    } else {
                        Ok("csv".to_string())
                    }
                }
                "parquet" => Ok("parquet".to_string()),
                "json" => Ok("json".to_string()),
                "jsonl" => Ok("jsonl".to_string()),
                "gz" => {
                    // Handle compressed files
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if stem.ends_with(".csv") {
                            Ok("csv".to_string())
                        } else if stem.ends_with(".tsv") {
                            Ok("tsv".to_string())
                        } else if stem.ends_with(".json") {
                            Ok("json".to_string())
                        } else if stem.ends_with(".jsonl") {
                            Ok("jsonl".to_string())
                        } else {
                            Ok("csv".to_string()) // Default
                        }
                    } else {
                        Ok("csv".to_string())
                    }
                }
                _ => Err(DataError::unsupported_format(format!(
                    "Unsupported file extension: {extension}"
                ))
                .into()),
            }
        } else {
            Err(
                DataError::unsupported_format("Cannot determine format from file path".to_string())
                    .into(),
            )
        }
    }

    /// Get or create a writer configuration for the given format
    fn get_writer_config(&self, format: &str) -> WriterConfig {
        self.format_configs.get(format).cloned().unwrap_or_else(|| {
            // Fallback to generic configuration
            WriterConfig {
                buffer_size: self.config.default_buffer_size,
                batch_size: self.config.default_batch_size,
                validate_on_write: self.config.enable_validation,
                encoding: "UTF-8".to_string(),
                field_separator: ',',
                include_headers: self.config.include_headers,
                compress_output: self.config.enable_compression,
                compression_level: self.config.default_compression_level,
                overwrite_existing: false,
                max_file_size: 0,
                float_precision: self.config.float_precision,
            }
        })
    }

    /// Create a writer optimized for the given file path
    pub fn create_optimized_writer(&self, path: &Path) -> Result<Box<dyn DataWriter>> {
        let format = self.determine_format(path)?;
        let mut config = self.get_writer_config(&format);

        // Enable compression for compressed file extensions
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            if extension == "gz" {
                config.compress_output = true;
            }
        }

        self.create_writer(&format, config)
    }

    /// Get supported output formats
    pub fn get_supported_formats(&self) -> Vec<String> {
        self.format_configs.keys().cloned().collect()
    }

    /// Add or update a format configuration
    pub fn set_format_config(&mut self, format: String, config: WriterConfig) {
        self.format_configs.insert(format, config);
    }

    /// Remove a format configuration
    pub fn remove_format_config(&mut self, format: &str) {
        self.format_configs.remove(format);
    }

    /// Check if compression is recommended for the given format
    pub fn is_compression_recommended(&self, format: &str) -> bool {
        match format {
            "parquet" => true,        // Parquet benefits greatly from compression
            "json" | "jsonl" => true, // JSON is verbose and compresses well
            "csv" | "tsv" => false,   // CSV/TSV are already compact
            _ => false,
        }
    }

    /// Get the optimal batch size for the given format
    pub fn get_optimal_batch_size(&self, format: &str) -> usize {
        match format {
            "parquet" => self.config.default_batch_size * 10, // Larger batches for columnar format
            "csv" | "tsv" => self.config.default_batch_size,
            "json" | "jsonl" => self.config.default_batch_size / 2, // Smaller batches for JSON
            _ => self.config.default_batch_size,
        }
    }
}

impl WriterFactory for DefaultWriterFactory {
    fn create_writer(&self, format: &str, config: WriterConfig) -> Result<Box<dyn DataWriter>> {
        match format.to_lowercase().as_str() {
            "csv" | "tsv" => Ok(Box::new(CsvDataWriter::new(config))),
            "parquet" => Ok(Box::new(ParquetDataWriter::new(config))),
            "json" | "jsonl" => {
                let mut writer = JsonDataWriter::new(config);
                if format == "jsonl" {
                    writer.set_array_format(false);
                }
                Ok(Box::new(writer))
            }
            _ => Err(DataError::unsupported_format(format!(
                "Unsupported writer format: {format}"
            ))
            .into()),
        }
    }

    fn create_series_writer(
        &self,
        format: &str,
        config: WriterConfig,
    ) -> Result<Box<dyn SeriesWriter>> {
        match format.to_lowercase().as_str() {
            "csv" | "tsv" => Ok(Box::new(CsvDataWriter::new(config))),
            "parquet" => Ok(Box::new(ParquetDataWriter::new(config))),
            "json" | "jsonl" => {
                let mut writer = JsonDataWriter::new(config);
                if format == "jsonl" {
                    writer.set_array_format(false);
                }
                Ok(Box::new(writer))
            }
            _ => Err(DataError::unsupported_format(format!(
                "Unsupported series writer format: {format}"
            ))
            .into()),
        }
    }

    fn create_observation_writer(
        &self,
        format: &str,
        config: WriterConfig,
    ) -> Result<Box<dyn ObservationWriter>> {
        match format.to_lowercase().as_str() {
            "csv" | "tsv" => Ok(Box::new(CsvDataWriter::new(config))),
            "parquet" => Ok(Box::new(ParquetDataWriter::new(config))),
            "json" | "jsonl" => {
                let mut writer = JsonDataWriter::new(config);
                if format == "jsonl" {
                    writer.set_array_format(false);
                }
                Ok(Box::new(writer))
            }
            _ => Err(DataError::unsupported_format(format!(
                "Unsupported observation writer format: {format}"
            ))
            .into()),
        }
    }

    fn create_lookup_writer(
        &self,
        format: &str,
        config: WriterConfig,
    ) -> Result<Box<dyn LookupWriter>> {
        match format.to_lowercase().as_str() {
            "csv" | "tsv" => Ok(Box::new(CsvDataWriter::new(config))),
            "parquet" => Ok(Box::new(ParquetDataWriter::new(config))),
            "json" | "jsonl" => {
                let mut writer = JsonDataWriter::new(config);
                if format == "jsonl" {
                    writer.set_array_format(false);
                }
                Ok(Box::new(writer))
            }
            _ => Err(DataError::unsupported_format(format!(
                "Unsupported lookup writer format: {format}"
            ))
            .into()),
        }
    }

    fn create_survey_writer(
        &self,
        format: &str,
        config: WriterConfig,
    ) -> Result<Box<dyn SurveyWriter>> {
        match format.to_lowercase().as_str() {
            "csv" | "tsv" => Ok(Box::new(CsvDataWriter::new(config))),
            "parquet" => Ok(Box::new(ParquetDataWriter::new(config))),
            "json" | "jsonl" => {
                let mut writer = JsonDataWriter::new(config);
                if format == "jsonl" {
                    writer.set_array_format(false);
                }
                Ok(Box::new(writer))
            }
            _ => Err(DataError::unsupported_format(format!(
                "Unsupported survey writer format: {format}"
            ))
            .into()),
        }
    }

    fn create_streaming_writer(
        &self,
        format: &str,
        config: WriterConfig,
    ) -> Result<Box<dyn StreamingWriter>> {
        match format.to_lowercase().as_str() {
            "json" | "jsonl" => {
                let mut writer = JsonDataWriter::new(config);
                if format == "jsonl" {
                    writer.set_array_format(false);
                }
                Ok(Box::new(writer))
            }
            _ => Err(DataError::unsupported_format(format!(
                "Streaming writer not supported for format: {format}"
            ))
            .into()),
        }
    }

    fn create_compressed_writer(
        &self,
        format: &str,
        config: WriterConfig,
    ) -> Result<Box<dyn CompressedWriter>> {
        match format.to_lowercase().as_str() {
            "parquet" => Ok(Box::new(ParquetDataWriter::new(config))),
            "json" | "jsonl" => {
                let mut writer = JsonDataWriter::new(config);
                if format == "jsonl" {
                    writer.set_array_format(false);
                }
                Ok(Box::new(writer))
            }
            _ => Err(DataError::unsupported_format(format!(
                "Compressed writer not supported for format: {format}"
            ))
            .into()),
        }
    }

    fn create_transactional_writer(
        &self,
        _format: &str,
        _config: WriterConfig,
    ) -> Result<Box<dyn TransactionalWriter>> {
        // Transactional writers are not yet implemented
        Err(
            DataError::not_implemented("Transactional writers are not yet implemented".to_string())
                .into(),
        )
    }

    fn supported_formats(&self) -> Vec<String> {
        self.get_supported_formats()
    }

    fn supports_compression(&self, format: &str) -> bool {
        match format.to_lowercase().as_str() {
            "parquet" | "json" | "jsonl" => true,
            "csv" | "tsv" => true, // Through gzip compression
            _ => false,
        }
    }

    fn supports_transactions(&self, _format: &str) -> bool {
        // Transactions are not yet implemented for any format
        false
    }
}

/// Builder for creating writer factory configurations
pub struct WriterFactoryConfigBuilder {
    config: WriterFactoryConfig,
}

impl WriterFactoryConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: WriterFactoryConfig::default(),
        }
    }

    /// Set the default buffer size
    pub fn default_buffer_size(mut self, size: usize) -> Self {
        self.config.default_buffer_size = size;
        self
    }

    /// Set the default batch size
    pub fn default_batch_size(mut self, size: usize) -> Self {
        self.config.default_batch_size = size;
        self
    }

    /// Enable or disable validation by default
    pub fn enable_validation(mut self, enable: bool) -> Self {
        self.config.enable_validation = enable;
        self
    }

    /// Enable or disable compression by default
    pub fn enable_compression(mut self, enable: bool) -> Self {
        self.config.enable_compression = enable;
        self
    }

    /// Set the default compression level
    pub fn default_compression_level(mut self, level: u8) -> Self {
        self.config.default_compression_level = level;
        self
    }

    /// Set the maximum number of errors to tolerate
    pub fn max_errors(mut self, max: usize) -> Self {
        self.config.max_errors = max;
        self
    }

    /// Enable or disable headers by default
    pub fn include_headers(mut self, include: bool) -> Self {
        self.config.include_headers = include;
        self
    }

    /// Set the default float precision
    pub fn float_precision(mut self, precision: usize) -> Self {
        self.config.float_precision = precision;
        self
    }

    /// Add a format-specific configuration
    pub fn format_config(mut self, format: String, config: WriterConfig) -> Self {
        self.config.format_configs.insert(format, config);
        self
    }

    /// Build the configuration
    pub fn build(self) -> WriterFactoryConfig {
        self.config
    }
}

impl Default for WriterFactoryConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_factory_creation() {
        let factory = DefaultWriterFactory::new();
        let supported_formats = factory.get_supported_formats();
        assert!(supported_formats.contains(&"csv".to_string()));
        assert!(supported_formats.contains(&"parquet".to_string()));
        assert!(supported_formats.contains(&"json".to_string()));
    }

    #[test]
    fn test_format_determination() {
        let factory = DefaultWriterFactory::new();

        assert_eq!(
            factory.determine_format(Path::new("test.csv")).unwrap(),
            "csv"
        );
        assert_eq!(
            factory.determine_format(Path::new("test.tsv")).unwrap(),
            "tsv"
        );
        assert_eq!(
            factory.determine_format(Path::new("test.parquet")).unwrap(),
            "parquet"
        );
        assert_eq!(
            factory.determine_format(Path::new("test.json")).unwrap(),
            "json"
        );
        assert_eq!(
            factory.determine_format(Path::new("test.jsonl")).unwrap(),
            "jsonl"
        );
        assert_eq!(
            factory.determine_format(Path::new("test.csv.gz")).unwrap(),
            "csv"
        );
    }

    #[test]
    fn test_compression_recommendations() {
        let factory = DefaultWriterFactory::new();

        assert!(factory.is_compression_recommended("parquet"));
        assert!(factory.is_compression_recommended("json"));
        assert!(!factory.is_compression_recommended("csv"));
    }

    #[test]
    fn test_optimal_batch_sizes() {
        let factory = DefaultWriterFactory::new();

        let default_batch = factory.config.default_batch_size;
        assert_eq!(
            factory.get_optimal_batch_size("parquet"),
            default_batch * 10
        );
        assert_eq!(factory.get_optimal_batch_size("csv"), default_batch);
        assert_eq!(factory.get_optimal_batch_size("json"), default_batch / 2);
    }

    #[test]
    fn test_writer_creation() {
        let factory = DefaultWriterFactory::new();
        let config = WriterConfig::default();

        // Test creating different writer types
        assert!(factory.create_writer("csv", config.clone()).is_ok());
        assert!(factory.create_writer("parquet", config.clone()).is_ok());
        assert!(factory.create_writer("json", config.clone()).is_ok());
        assert!(factory.create_writer("unsupported", config).is_err());
    }

    #[test]
    fn test_config_builder() {
        let config = WriterFactoryConfigBuilder::new()
            .default_buffer_size(128 * 1024)
            .default_batch_size(2000)
            .enable_validation(false)
            .enable_compression(true)
            .default_compression_level(9)
            .max_errors(500)
            .include_headers(false)
            .float_precision(4)
            .build();

        assert_eq!(config.default_buffer_size, 128 * 1024);
        assert_eq!(config.default_batch_size, 2000);
        assert!(!config.enable_validation);
        assert!(config.enable_compression);
        assert_eq!(config.default_compression_level, 9);
        assert_eq!(config.max_errors, 500);
        assert!(!config.include_headers);
        assert_eq!(config.float_precision, 4);
    }

    #[test]
    fn test_format_support() {
        let factory = DefaultWriterFactory::new();

        assert!(factory.supports_compression("parquet"));
        assert!(factory.supports_compression("json"));
        assert!(factory.supports_compression("csv"));
        assert!(!factory.supports_compression("unsupported"));

        // Transactions not yet implemented
        assert!(!factory.supports_transactions("csv"));
        assert!(!factory.supports_transactions("parquet"));
    }
}
