//! Data writer module for BLS data processing
//!
//! This module provides comprehensive writing capabilities for BLS data files
//! with support for different output formats, writing strategies, and performance
//! optimizations.

pub mod traits;
pub mod csv_writer;
pub mod parquet_writer;
pub mod json_writer;
pub mod factory;

// Re-export public types and traits
pub use traits::{
    DataWriter, SeriesWriter, ObservationWriter, LookupWriter, SurveyWriter,
    StreamingWriter, CompressedWriter, TransactionalWriter, WriterFactory,
    WriterConfig, WriteStats, OutputMetadata, CompressionInfo, SchemaInfo,
};

pub use csv_writer::CsvDataWriter;
pub use parquet_writer::ParquetDataWriter;
pub use json_writer::JsonDataWriter;
pub use factory::{DefaultWriterFactory, WriterFactoryConfig, WriterFactoryConfigBuilder};

/// Create a default writer factory
pub fn create_default_factory() -> DefaultWriterFactory {
    DefaultWriterFactory::new()
}

/// Create a writer factory with custom configuration
pub fn create_factory_with_config(config: WriterFactoryConfig) -> DefaultWriterFactory {
    DefaultWriterFactory::with_config(config)
}

/// Convenience function to create an optimized writer for a file
pub fn create_optimized_writer(path: &std::path::Path) -> crate::error::types::Result<Box<dyn DataWriter>> {
    let factory = create_default_factory();
    factory.create_optimized_writer(path)
}

/// Create a writer for a specific format
pub fn create_writer_for_format(format: &str, config: WriterConfig) -> crate::error::types::Result<Box<dyn DataWriter>> {
    let factory = create_default_factory();
    factory.create_writer(format, config)
}

/// Get all supported output formats
pub fn get_supported_formats() -> Vec<String> {
    let factory = create_default_factory();
    factory.get_supported_formats()
}

/// Check if a format supports compression
pub fn format_supports_compression(format: &str) -> bool {
    let factory = create_default_factory();
    factory.supports_compression(format)
}

/// Check if compression is recommended for a format
pub fn is_compression_recommended(format: &str) -> bool {
    let factory = create_default_factory();
    factory.is_compression_recommended(format)
}

/// Get the optimal batch size for a format
pub fn get_optimal_batch_size(format: &str) -> usize {
    let factory = create_default_factory();
    factory.get_optimal_batch_size(format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_module_exports() {
        // Test that we can create a factory
        let factory = create_default_factory();
        assert!(!factory.get_supported_formats().is_empty());
    }

    #[test]
    fn test_convenience_functions() {
        let formats = get_supported_formats();
        assert!(formats.contains(&"csv".to_string()));
        assert!(formats.contains(&"parquet".to_string()));
        assert!(formats.contains(&"json".to_string()));

        assert!(format_supports_compression("parquet"));
        assert!(is_compression_recommended("json"));
        assert!(get_optimal_batch_size("parquet") > get_optimal_batch_size("csv"));
    }

    #[test]
    fn test_writer_creation() {
        let config = WriterConfig::default();
        
        // Test creating writers for different formats
        assert!(create_writer_for_format("csv", config.clone()).is_ok());
        assert!(create_writer_for_format("parquet", config.clone()).is_ok());
        assert!(create_writer_for_format("json", config.clone()).is_ok());
        assert!(create_writer_for_format("unsupported", config).is_err());
    }

    #[test]
    fn test_optimized_writer_creation() {
        // Test creating optimized writers based on file paths
        let csv_path = Path::new("test.csv");
        assert!(create_optimized_writer(csv_path).is_ok());
        
        let parquet_path = Path::new("test.parquet");
        assert!(create_optimized_writer(parquet_path).is_ok());
        
        let json_path = Path::new("test.json");
        assert!(create_optimized_writer(json_path).is_ok());
    }
}