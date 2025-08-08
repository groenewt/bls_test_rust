//! Writer trait definitions for BLS data processing
//!
//! This module defines the core traits for writing BLS data files in various formats.
//! The traits are designed to be flexible, performant, and support different writing
//! strategies based on output format and performance requirements.

use std::path::Path;
use async_trait::async_trait;
use serde::Serialize;
use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::error::types::{DataError, Result};

/// Configuration for writer behavior
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Buffer size for writing operations (in bytes)
    pub buffer_size: usize,
    /// Maximum number of records to write in a single batch
    pub batch_size: usize,
    /// Whether to validate data before writing
    pub validate_on_write: bool,
    /// Output file encoding
    pub encoding: String,
    /// Field separator for delimited files
    pub field_separator: char,
    /// Whether to include headers in output files
    pub include_headers: bool,
    /// Whether to compress output files
    pub compress_output: bool,
    /// Compression level (0-9, where applicable)
    pub compression_level: u8,
    /// Whether to overwrite existing files
    pub overwrite_existing: bool,
    /// Maximum file size before splitting (in bytes, 0 = no limit)
    pub max_file_size: u64,
    /// Number format precision for floating point values
    pub float_precision: usize,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            buffer_size: 64 * 1024, // 64KB
            batch_size: 1000,
            validate_on_write: true,
            encoding: "UTF-8".to_string(),
            field_separator: '\t',
            include_headers: true,
            compress_output: false,
            compression_level: 6,
            overwrite_existing: false,
            max_file_size: 0, // No limit
            float_precision: 6,
        }
    }
}

/// Statistics collected during writing operations
#[derive(Debug, Default, Clone)]
pub struct WriteStats {
    /// Total number of records written
    pub records_written: u64,
    /// Number of bytes written
    pub bytes_written: u64,
    /// Number of files created
    pub files_created: u64,
    /// Number of errors encountered
    pub errors_encountered: u64,
    /// Number of records skipped due to validation failures
    pub records_skipped: u64,
    /// Time taken for writing operation (in milliseconds)
    pub write_time_ms: u64,
    /// Compression ratio (if compression is enabled)
    pub compression_ratio: f64,
}

/// Core trait for writing BLS data files
#[async_trait]
pub trait DataWriter: Send + Sync {
    /// Writer configuration
    fn config(&self) -> &WriterConfig;
    
    /// Get writing statistics
    fn stats(&self) -> &WriteStats;
    
    /// Reset statistics
    fn reset_stats(&mut self);
    
    /// Check if the writer can handle the given output path
    fn can_write(&self, path: &Path) -> Result<bool>;
    
    /// Open a file for writing
    async fn open(&mut self, path: &Path) -> Result<()>;
    
    /// Close the currently open file
    async fn close(&mut self) -> Result<()>;
    
    /// Flush any buffered data to disk
    async fn flush(&mut self) -> Result<()>;
    
    /// Check if a file is currently open
    fn is_open(&self) -> bool;
    
    /// Get the path of the currently open file
    fn current_file(&self) -> Option<&Path>;
    
    /// Get the supported file extensions
    fn supported_extensions(&self) -> Vec<String>;
}

/// Trait for writing series data files
#[async_trait]
pub trait SeriesWriter: DataWriter {
    /// Write a single series record
    async fn write_series(&mut self, series: &Series) -> Result<()>;
    
    /// Write multiple series records
    async fn write_series_batch(&mut self, series: &[Series]) -> Result<()>;
    
    /// Write all series from an iterator
    async fn write_all_series<I>(&mut self, series: I) -> Result<()>
    where
        I: Iterator<Item = Series> + Send,
        I::Item: Send;
    
    /// Write series with a custom serializer
    async fn write_series_with_serializer<F>(&mut self, series: &Series, serializer: F) -> Result<()>
    where
        F: Fn(&Series) -> Result<String> + Send + Sync;
}

/// Trait for writing observation data files
#[async_trait]
pub trait ObservationWriter: DataWriter {
    /// Write a single observation record
    async fn write_observation(&mut self, observation: &Observation) -> Result<()>;
    
    /// Write multiple observation records
    async fn write_observations_batch(&mut self, observations: &[Observation]) -> Result<()>;
    
    /// Write all observations from an iterator
    async fn write_all_observations<I>(&mut self, observations: I) -> Result<()>
    where
        I: Iterator<Item = Observation> + Send,
        I::Item: Send;
    
    /// Write observations for a specific series
    async fn write_observations_for_series(&mut self, series_id: &str, observations: &[Observation]) -> Result<()>;
    
    /// Write observations with a custom serializer
    async fn write_observations_with_serializer<F>(&mut self, observations: &[Observation], serializer: F) -> Result<()>
    where
        F: Fn(&Observation) -> Result<String> + Send + Sync;
}

/// Trait for writing lookup table files
#[async_trait]
pub trait LookupWriter: DataWriter {
    /// Write a single lookup record
    async fn write_lookup(&mut self, lookup: &Lookup) -> Result<()>;
    
    /// Write multiple lookup records
    async fn write_lookups_batch(&mut self, lookups: &[Lookup]) -> Result<()>;
    
    /// Write all lookups from an iterator
    async fn write_all_lookups<I>(&mut self, lookups: I) -> Result<()>
    where
        I: Iterator<Item = Lookup> + Send,
        I::Item: Send;
    
    /// Write lookups with a custom serializer
    async fn write_lookups_with_serializer<F>(&mut self, lookups: &[Lookup], serializer: F) -> Result<()>
    where
        F: Fn(&Lookup) -> Result<String> + Send + Sync;
}

/// Trait for writing survey metadata files
#[async_trait]
pub trait SurveyWriter: DataWriter {
    /// Write survey metadata
    async fn write_survey(&mut self, survey: &Survey) -> Result<()>;
    
    /// Write survey with custom formatting
    async fn write_survey_with_format<F>(&mut self, survey: &Survey, formatter: F) -> Result<()>
    where
        F: Fn(&Survey) -> Result<String> + Send + Sync;
}

/// Trait for streaming large datasets efficiently
#[async_trait]
pub trait StreamingWriter: DataWriter {
    /// Start a streaming write session
    async fn start_stream(&mut self) -> Result<()>;
    
    /// Write a chunk of data to the stream
    async fn write_chunk(&mut self, data: &[u8]) -> Result<()>;
    
    /// Write a record to the stream with custom serialization
    async fn write_record<T, F>(&mut self, record: &T, serializer: F) -> Result<()>
    where
        T: Send + Sync,
        F: Fn(&T) -> Result<Vec<u8>> + Send + Sync;
    
    /// End the streaming write session
    async fn end_stream(&mut self) -> Result<()>;
}

/// Trait for compressed file writing
#[async_trait]
pub trait CompressedWriter: DataWriter {
    /// Set compression level (0-9)
    fn set_compression_level(&mut self, level: u8) -> Result<()>;
    
    /// Get current compression level
    fn compression_level(&self) -> u8;
    
    /// Get compression statistics
    fn compression_stats(&self) -> (u64, u64, f64); // (uncompressed, compressed, ratio)
}

/// Trait for transactional writing operations
#[async_trait]
pub trait TransactionalWriter: DataWriter {
    /// Begin a transaction
    async fn begin_transaction(&mut self) -> Result<()>;
    
    /// Commit the current transaction
    async fn commit_transaction(&mut self) -> Result<()>;
    
    /// Rollback the current transaction
    async fn rollback_transaction(&mut self) -> Result<()>;
    
    /// Check if a transaction is active
    fn has_active_transaction(&self) -> bool;
}

/// Factory trait for creating writers
pub trait WriterFactory: Send + Sync {
    /// Create a writer for the given format and configuration
    fn create_writer(&self, format: &str, config: WriterConfig) -> Result<Box<dyn DataWriter>>;
    
    /// Create a series writer
    fn create_series_writer(&self, format: &str, config: WriterConfig) -> Result<Box<dyn SeriesWriter>>;
    
    /// Create an observation writer
    fn create_observation_writer(&self, format: &str, config: WriterConfig) -> Result<Box<dyn ObservationWriter>>;
    
    /// Create a lookup writer
    fn create_lookup_writer(&self, format: &str, config: WriterConfig) -> Result<Box<dyn LookupWriter>>;
    
    /// Create a survey writer
    fn create_survey_writer(&self, format: &str, config: WriterConfig) -> Result<Box<dyn SurveyWriter>>;
    
    /// Create a streaming writer
    fn create_streaming_writer(&self, format: &str, config: WriterConfig) -> Result<Box<dyn StreamingWriter>>;
    
    /// Create a compressed writer
    fn create_compressed_writer(&self, format: &str, config: WriterConfig) -> Result<Box<dyn CompressedWriter>>;
    
    /// Create a transactional writer
    fn create_transactional_writer(&self, format: &str, config: WriterConfig) -> Result<Box<dyn TransactionalWriter>>;
    
    /// Get supported output formats
    fn supported_formats(&self) -> Vec<String>;
    
    /// Check if a format supports compression
    fn supports_compression(&self, format: &str) -> bool;
    
    /// Check if a format supports transactions
    fn supports_transactions(&self, format: &str) -> bool;
}

/// Metadata for output files
#[derive(Debug, Clone, Serialize)]
pub struct OutputMetadata {
    /// File format
    pub format: String,
    /// File size in bytes
    pub file_size: u64,
    /// Number of records
    pub record_count: u64,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Compression information
    pub compression: Option<CompressionInfo>,
    /// Schema information
    pub schema: Option<SchemaInfo>,
    /// Custom metadata
    pub custom: std::collections::HashMap<String, String>,
}

/// Compression information
#[derive(Debug, Clone, Serialize)]
pub struct CompressionInfo {
    /// Compression algorithm
    pub algorithm: String,
    /// Compression level
    pub level: u8,
    /// Original size
    pub original_size: u64,
    /// Compressed size
    pub compressed_size: u64,
    /// Compression ratio
    pub ratio: f64,
}

/// Schema information
#[derive(Debug, Clone, Serialize)]
pub struct SchemaInfo {
    /// Field names
    pub fields: Vec<String>,
    /// Field types
    pub field_types: Vec<String>,
    /// Field descriptions
    pub field_descriptions: Option<Vec<String>>,
}

// Enum wrappers for dyn compatibility
// These enums allow us to use concrete types instead of trait objects
// while still providing dynamic dispatch capabilities

use crate::data::writer::csv_writer::CsvDataWriter;
use crate::data::writer::parquet_writer::ParquetDataWriter;
use crate::data::writer::json_writer::JsonDataWriter;

/// Enum wrapper for SeriesWriter implementations
#[derive(Debug)]
pub enum SeriesWriterEnum {
    Csv(CsvDataWriter),
    Parquet(ParquetDataWriter),
    Json(JsonDataWriter),
}

/// Enum wrapper for ObservationWriter implementations
#[derive(Debug)]
pub enum ObservationWriterEnum {
    Csv(CsvDataWriter),
    Parquet(ParquetDataWriter),
    Json(JsonDataWriter),
}

/// Enum wrapper for LookupWriter implementations
#[derive(Debug)]
pub enum LookupWriterEnum {
    Csv(CsvDataWriter),
    Parquet(ParquetDataWriter),
    Json(JsonDataWriter),
}

/// Enum wrapper for SurveyWriter implementations
#[derive(Debug)]
pub enum SurveyWriterEnum {
    Csv(CsvDataWriter),
    Parquet(ParquetDataWriter),
    Json(JsonDataWriter),
}

/// Enum wrapper for StreamingWriter implementations
#[derive(Debug)]
pub enum StreamingWriterEnum {
    Json(JsonDataWriter),
}

/// Enum wrapper for CompressedWriter implementations
#[derive(Debug)]
pub enum CompressedWriterEnum {
    Parquet(ParquetDataWriter),
    Json(JsonDataWriter),
}

// Implement DataWriter for all enum wrappers
#[async_trait]
impl DataWriter for SeriesWriterEnum {
    fn config(&self) -> &WriterConfig {
        match self {
            SeriesWriterEnum::Csv(w) => w.config(),
            SeriesWriterEnum::Parquet(w) => w.config(),
            SeriesWriterEnum::Json(w) => w.config(),
        }
    }

    fn stats(&self) -> &WriteStats {
        match self {
            SeriesWriterEnum::Csv(w) => w.stats(),
            SeriesWriterEnum::Parquet(w) => w.stats(),
            SeriesWriterEnum::Json(w) => w.stats(),
        }
    }

    fn reset_stats(&mut self) {
        match self {
            SeriesWriterEnum::Csv(w) => w.reset_stats(),
            SeriesWriterEnum::Parquet(w) => w.reset_stats(),
            SeriesWriterEnum::Json(w) => w.reset_stats(),
        }
    }

    fn can_write(&self, path: &Path) -> Result<bool> {
        match self {
            SeriesWriterEnum::Csv(w) => w.can_write(path),
            SeriesWriterEnum::Parquet(w) => w.can_write(path),
            SeriesWriterEnum::Json(w) => w.can_write(path),
        }
    }

    async fn open(&mut self, path: &Path) -> Result<()> {
        match self {
            SeriesWriterEnum::Csv(w) => w.open(path).await,
            SeriesWriterEnum::Parquet(w) => w.open(path).await,
            SeriesWriterEnum::Json(w) => w.open(path).await,
        }
    }

    async fn close(&mut self) -> Result<()> {
        match self {
            SeriesWriterEnum::Csv(w) => w.close().await,
            SeriesWriterEnum::Parquet(w) => w.close().await,
            SeriesWriterEnum::Json(w) => w.close().await,
        }
    }

    async fn flush(&mut self) -> Result<()> {
        match self {
            SeriesWriterEnum::Csv(w) => w.flush().await,
            SeriesWriterEnum::Parquet(w) => w.flush().await,
            SeriesWriterEnum::Json(w) => w.flush().await,
        }
    }

    fn is_open(&self) -> bool {
        match self {
            SeriesWriterEnum::Csv(w) => w.is_open(),
            SeriesWriterEnum::Parquet(w) => w.is_open(),
            SeriesWriterEnum::Json(w) => w.is_open(),
        }
    }

    fn current_file(&self) -> Option<&Path> {
        match self {
            SeriesWriterEnum::Csv(w) => w.current_file(),
            SeriesWriterEnum::Parquet(w) => w.current_file(),
            SeriesWriterEnum::Json(w) => w.current_file(),
        }
    }

    fn supported_extensions(&self) -> Vec<String> {
        match self {
            SeriesWriterEnum::Csv(w) => w.supported_extensions(),
            SeriesWriterEnum::Parquet(w) => w.supported_extensions(),
            SeriesWriterEnum::Json(w) => w.supported_extensions(),
        }
    }
}

// Implement SeriesWriter for SeriesWriterEnum
#[async_trait]
impl SeriesWriter for SeriesWriterEnum {
    async fn write_series(&mut self, series: &Series) -> Result<()> {
        match self {
            SeriesWriterEnum::Csv(w) => w.write_series(series).await,
            SeriesWriterEnum::Parquet(w) => w.write_series(series).await,
            SeriesWriterEnum::Json(w) => w.write_series(series).await,
        }
    }

    async fn write_series_batch(&mut self, series: &[Series]) -> Result<()> {
        match self {
            SeriesWriterEnum::Csv(w) => w.write_series_batch(series).await,
            SeriesWriterEnum::Parquet(w) => w.write_series_batch(series).await,
            SeriesWriterEnum::Json(w) => w.write_series_batch(series).await,
        }
    }

    async fn write_all_series<I>(&mut self, series: I) -> Result<()>
    where
        I: Iterator<Item = Series> + Send,
        I::Item: Send,
    {
        match self {
            SeriesWriterEnum::Csv(w) => w.write_all_series(series).await,
            SeriesWriterEnum::Parquet(w) => w.write_all_series(series).await,
            SeriesWriterEnum::Json(w) => w.write_all_series(series).await,
        }
    }

    async fn write_series_with_serializer<F>(&mut self, series: &Series, serializer: F) -> Result<()>
    where
        F: Fn(&Series) -> Result<String> + Send + Sync,
    {
        match self {
            SeriesWriterEnum::Csv(w) => w.write_series_with_serializer(series, serializer).await,
            SeriesWriterEnum::Parquet(w) => w.write_series_with_serializer(series, serializer).await,
            SeriesWriterEnum::Json(w) => w.write_series_with_serializer(series, serializer).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_config_default() {
        let config = WriterConfig::default();
        assert_eq!(config.buffer_size, 64 * 1024);
        assert_eq!(config.batch_size, 1000);
        assert!(config.validate_on_write);
        assert_eq!(config.encoding, "UTF-8");
        assert_eq!(config.field_separator, '\t');
        assert!(config.include_headers);
        assert!(!config.compress_output);
        assert_eq!(config.compression_level, 6);
        assert!(!config.overwrite_existing);
        assert_eq!(config.max_file_size, 0);
        assert_eq!(config.float_precision, 6);
    }

    #[test]
    fn test_write_stats_default() {
        let stats = WriteStats::default();
        assert_eq!(stats.records_written, 0);
        assert_eq!(stats.bytes_written, 0);
        assert_eq!(stats.files_created, 0);
        assert_eq!(stats.errors_encountered, 0);
        assert_eq!(stats.records_skipped, 0);
        assert_eq!(stats.write_time_ms, 0);
        assert_eq!(stats.compression_ratio, 0.0);
    }

    #[test]
    fn test_output_metadata_creation() {
        let metadata = OutputMetadata {
            format: "CSV".to_string(),
            file_size: 1024,
            record_count: 100,
            created_at: chrono::Utc::now(),
            compression: None,
            schema: None,
            custom: std::collections::HashMap::new(),
        };
        
        assert_eq!(metadata.format, "CSV");
        assert_eq!(metadata.file_size, 1024);
        assert_eq!(metadata.record_count, 100);
    }
}