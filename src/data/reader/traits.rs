//! Reader trait definitions for BLS data processing
//!
//! This module defines the core traits for reading BLS data files in various formats.
//! The traits are designed to be flexible, performant, and support different reading
//! strategies based on file size and processing requirements.

use std::any::Any;
use std::io::{BufRead, Read, Seek};
use std::path::Path;
use async_trait::async_trait;
use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::error::types::{DataError, Result};

/// Configuration for reader behavior
#[derive(Debug, Clone)]
pub struct ReaderConfig {
    /// Buffer size for reading operations (in bytes)
    pub buffer_size: usize,
    /// Maximum number of records to read in a single batch
    pub batch_size: usize,
    /// Whether to validate data during reading
    pub validate_on_read: bool,
    /// Whether to use memory mapping for large files
    pub use_memory_mapping: bool,
    /// Encoding of the input files
    pub encoding: String,
    /// Field separator for delimited files
    pub field_separator: char,
    /// Whether to skip malformed records
    pub skip_malformed: bool,
    /// Maximum number of errors to tolerate before aborting
    pub max_errors: usize,
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self {
            buffer_size: 64 * 1024, // 64KB
            batch_size: 1000,
            validate_on_read: true,
            use_memory_mapping: false,
            encoding: "UTF-8".to_string(),
            field_separator: '\t',
            skip_malformed: false,
            max_errors: 100,
        }
    }
}

/// Statistics collected during reading operations
#[derive(Debug, Default, Clone)]
pub struct ReadStats {
    /// Total number of records read
    pub records_read: u64,
    /// Number of bytes processed
    pub bytes_processed: u64,
    /// Number of errors encountered
    pub errors_encountered: u64,
    /// Number of records skipped due to validation failures
    pub records_skipped: u64,
    /// Time taken for reading operation (in milliseconds)
    pub read_time_ms: u64,
}

/// Core trait for reading BLS data files
#[async_trait]
pub trait DataReader: Send + Sync + std::fmt::Debug {
    /// Read configuration
    fn config(&self) -> &ReaderConfig;
    
    /// Get reading statistics
    fn stats(&self) -> &ReadStats;
    
    /// Reset statistics
    fn reset_stats(&mut self);
    
    /// Check if the reader can handle the given file
    fn can_read(&self, path: &Path) -> Result<bool>;
    
    /// Open a file for reading
    async fn open(&mut self, path: &Path) -> Result<()>;
    
    /// Close the currently open file
    async fn close(&mut self) -> Result<()>;
    
    /// Check if a file is currently open
    fn is_open(&self) -> bool;
    
    /// Get the path of the currently open file
    fn current_file(&self) -> Option<&Path>;
    
    /// Enable downcasting to concrete types
    fn as_any(&self) -> &dyn Any;
    
    /// Enable mutable downcasting to concrete types
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Trait for reading series data files
#[async_trait]
pub trait SeriesReader: DataReader {
    /// Read all series from the file
    async fn read_all_series(&mut self) -> Result<Vec<Series>>;
    
    /// Read series in batches
    async fn read_series_batch(&mut self, batch_size: usize) -> Result<Vec<Series>>;
    
    /// Read a specific series by ID
    async fn read_series_by_id(&mut self, series_id: &str) -> Result<Option<Series>>;
    
    /// Count total number of series in the file
    async fn count_series(&mut self) -> Result<u64>;
}

/// Trait for iterating over series data with callbacks
#[async_trait]
pub trait SeriesIterator {
    /// Iterate over series with a callback
    async fn for_each_series<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(Series) -> Result<()> + Send + Sync;
}

/// Trait for iterating over observations with callbacks
#[async_trait]
pub trait ObservationIterator {
    /// Iterate over observations with a callback
    async fn for_each_observation<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(Observation) -> Result<()> + Send + Sync;
}

/// Trait for iterating over lookups with callbacks
#[async_trait]
pub trait LookupIterator {
    /// Iterate over lookup entries with a callback
    async fn for_each_lookup<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(Lookup) -> Result<()> + Send + Sync;
}

/// Trait for streaming operations with generic processors
#[async_trait]
pub trait StreamingProcessor {
    /// Stream data with a processing function
    async fn stream_with_processor<F, T>(&mut self, processor: F) -> Result<Vec<T>>
    where
        F: Fn(&str) -> Result<T> + Send + Sync,
        T: Send + Sync;
    
    /// Stream data line by line
    async fn stream_lines<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(&str) -> Result<()> + Send + Sync;
}

/// Trait for reading observation data files
#[async_trait]
pub trait ObservationReader: DataReader {
    /// Read all observations from the file
    async fn read_all_observations(&mut self) -> Result<Vec<Observation>>;
    
    /// Read observations in batches
    async fn read_observations_batch(&mut self, batch_size: usize) -> Result<Vec<Observation>>;
    
    /// Read observations for a specific series
    async fn read_observations_for_series(&mut self, series_id: &str) -> Result<Vec<Observation>>;
    
    /// Read observations within a date range
    async fn read_observations_by_date_range(
        &mut self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<Observation>>;
    
    /// Count total number of observations in the file
    async fn count_observations(&mut self) -> Result<u64>;
}

/// Trait for reading lookup table files
#[async_trait]
pub trait LookupReader: DataReader {
    /// Read all lookup entries from the file
    async fn read_all_lookups(&mut self) -> Result<Vec<Lookup>>;
    
    /// Read lookup entries in batches
    async fn read_lookups_batch(&mut self, batch_size: usize) -> Result<Vec<Lookup>>;
    
    /// Read a specific lookup entry by code
    async fn read_lookup_by_code(&mut self, code: &str) -> Result<Option<Lookup>>;
    
    /// Count total number of lookup entries in the file
    async fn count_lookups(&mut self) -> Result<u64>;
}

/// Trait for reading survey metadata files
#[async_trait]
pub trait SurveyReader: DataReader {
    /// Read survey metadata
    async fn read_survey(&mut self) -> Result<Survey>;
    
    /// Validate survey structure
    async fn validate_survey_structure(&mut self) -> Result<bool>;
}

/// Trait for streaming large files efficiently
#[async_trait]
pub trait StreamingReader: DataReader {
    /// Stream data line by line (returns number of lines processed)
    async fn stream_lines(&mut self) -> Result<u64>;
}

/// Trait for memory-mapped file reading
pub trait MemoryMappedReader: DataReader {
    /// Get a memory-mapped view of the file
    fn memory_map(&self) -> Result<&[u8]>;
    
    /// Get a slice of the memory-mapped file
    fn slice(&self, start: usize, len: usize) -> Result<&[u8]>;
    
    /// Search for a pattern in the memory-mapped file
    fn find_pattern(&self, pattern: &[u8]) -> Result<Vec<usize>>;
}

/// Factory trait for creating readers
pub trait ReaderFactory: Send + Sync {
    /// Create a reader for the given file type and path
    fn create_reader(&self, file_type: &str, config: ReaderConfig) -> Result<Box<dyn DataReader>>;
    
    /// Create a series reader
    fn create_series_reader(&self, config: ReaderConfig) -> Result<Box<dyn SeriesReader>>;
    
    /// Create an observation reader
    fn create_observation_reader(&self, config: ReaderConfig) -> Result<Box<dyn ObservationReader>>;
    
    /// Create a lookup reader
    fn create_lookup_reader(&self, config: ReaderConfig) -> Result<Box<dyn LookupReader>>;
    
    /// Create a survey reader
    fn create_survey_reader(&self, config: ReaderConfig) -> Result<Box<dyn SurveyReader>>;
    
    /// Create a streaming reader
    fn create_streaming_reader(&self, config: ReaderConfig) -> Result<Box<dyn StreamingReader>>;
    
    /// Create a memory-mapped reader
    fn create_memory_mapped_reader(&self, config: ReaderConfig) -> Result<Box<dyn MemoryMappedReader>>;
    
    /// Get supported file types
    fn supported_file_types(&self) -> Vec<String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_config_default() {
        let config = ReaderConfig::default();
        assert_eq!(config.buffer_size, 64 * 1024);
        assert_eq!(config.batch_size, 1000);
        assert!(config.validate_on_read);
        assert!(!config.use_memory_mapping);
        assert_eq!(config.encoding, "UTF-8");
        assert_eq!(config.field_separator, '\t');
        assert!(!config.skip_malformed);
        assert_eq!(config.max_errors, 100);
    }

    #[test]
    fn test_read_stats_default() {
        let stats = ReadStats::default();
        assert_eq!(stats.records_read, 0);
        assert_eq!(stats.bytes_processed, 0);
        assert_eq!(stats.errors_encountered, 0);
        assert_eq!(stats.records_skipped, 0);
        assert_eq!(stats.read_time_ms, 0);
    }
}