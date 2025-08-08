//! # Output Format Module
//!
//! This module contains format-specific implementations for different output formats.
//! Each format provides both a FormatWriter and an OutputGenerator implementation.

pub mod csv;
pub mod json;
pub mod parquet;

// Re-export format implementations
pub use csv::{CsvWriter, CsvOutputGenerator};
pub use json::{JsonWriter, JsonOutputGenerator};
pub use parquet::{ParquetWriter, ParquetOutputGenerator};

use crate::output::traits::{FormatWriter, OutputGenerator};
use crate::error::types::{ProcessingError, Result};

/// Create a format writer for the specified format
pub fn create_format_writer(format: &str) -> Result<Box<dyn FormatWriter>> {
    match format.to_lowercase().as_str() {
        "csv" => Ok(Box::new(CsvWriter::new())),
        "json" => Ok(Box::new(JsonWriter::new())),
        "parquet" => Ok(Box::new(ParquetWriter::new())),
        _ => Err(ProcessingError::UnsupportedOperation {
            operation: format!("format_writer_{}", format),
            message: format!("Unsupported format: {}", format),
        }.into()),
    }
}

/// Create an output generator for the specified format
pub fn create_format_generator(format: &str) -> Result<Box<dyn OutputGenerator>> {
    match format.to_lowercase().as_str() {
        "csv" => Ok(Box::new(CsvOutputGenerator::new())),
        "json" => Ok(Box::new(JsonOutputGenerator::new())),
        "parquet" => Ok(Box::new(ParquetOutputGenerator::new())),
        _ => Err(ProcessingError::UnsupportedOperation {
            operation: format!("format_generator_{}", format),
            message: format!("Unsupported format: {}", format),
        }.into()),
    }
}

/// Get list of all supported formats
pub fn supported_formats() -> Vec<String> {
    vec![
        "csv".to_string(),
        "json".to_string(),
        "parquet".to_string(),
    ]
}

/// Check if a format is supported
pub fn is_format_supported(format: &str) -> bool {
    supported_formats().contains(&format.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_formats() {
        let formats = supported_formats();
        assert!(formats.contains(&"csv".to_string()));
        assert!(formats.contains(&"json".to_string()));
        assert!(formats.contains(&"parquet".to_string()));
    }

    #[test]
    fn test_is_format_supported() {
        assert!(is_format_supported("csv"));
        assert!(is_format_supported("CSV"));
        assert!(is_format_supported("json"));
        assert!(is_format_supported("parquet"));
        assert!(!is_format_supported("unknown"));
    }

    #[test]
    fn test_create_csv_writer() {
        let writer = create_format_writer("csv");
        assert!(writer.is_ok());
        assert_eq!(writer.unwrap().format_name(), "csv");
    }

    #[test]
    fn test_create_json_writer() {
        let writer = create_format_writer("json");
        assert!(writer.is_ok());
        assert_eq!(writer.unwrap().format_name(), "json");
    }

    #[test]
    fn test_create_parquet_writer() {
        let writer = create_format_writer("parquet");
        assert!(writer.is_ok());
        assert_eq!(writer.unwrap().format_name(), "parquet");
    }

    #[test]
    fn test_create_unsupported_writer() {
        let writer = create_format_writer("unknown");
        assert!(writer.is_err());
    }

    #[test]
    fn test_create_csv_generator() {
        let generator = create_format_generator("csv");
        assert!(generator.is_ok());
        assert_eq!(generator.unwrap().name(), "csv_generator");
    }

    #[test]
    fn test_create_unsupported_generator() {
        let generator = create_format_generator("unknown");
        assert!(generator.is_err());
    }
}