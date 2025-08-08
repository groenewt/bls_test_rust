//! # Output Trait Interfaces
//!
//! This module defines the trait interfaces for output generation across
//! different formats and destinations.

use std::collections::HashMap;
use std::path::Path;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::processing::traits::ProcessedData;
use crate::error::types::Result;

/// Main trait for output generators
#[async_trait]
pub trait OutputGenerator: Send + Sync {
    /// Get the name of this output generator
    fn name(&self) -> &str;

    /// Get the description of this output generator
    fn description(&self) -> &str;

    /// Get supported output formats
    fn supported_formats(&self) -> Vec<String>;

    /// Check if this generator can handle the given format
    fn can_handle_format(&self, format: &str) -> bool {
        self.supported_formats().contains(&format.to_lowercase())
    }

    /// Generate output from processed data
    async fn generate(&mut self, data: ProcessedData, config: OutputConfig) -> Result<OutputResult>;

    /// Validate the output configuration
    fn validate_config(&self, config: &OutputConfig) -> Result<()>;

    /// Get output statistics
    fn stats(&self) -> OutputStats;

    /// Reset output statistics
    fn reset_stats(&mut self);
}

/// Configuration for output generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output format (csv, parquet, json, etc.)
    pub format: String,
    /// Output destination path
    pub destination: String,
    /// Compression settings
    pub compression: Option<CompressionConfig>,
    /// Partitioning settings
    pub partitioning: Option<PartitioningConfig>,
    /// Format-specific options
    pub format_options: HashMap<String, String>,
    /// Metadata to include in output
    pub metadata: HashMap<String, String>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: "csv".to_string(),
            destination: "output".to_string(),
            compression: None,
            partitioning: None,
            format_options: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Compression algorithm (gzip, snappy, lz4, etc.)
    pub algorithm: String,
    /// Compression level (if applicable)
    pub level: Option<u32>,
    /// Enable compression
    pub enabled: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: "gzip".to_string(),
            level: Some(6),
            enabled: false,
        }
    }
}

/// Partitioning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitioningConfig {
    /// Fields to partition by
    pub partition_by: Vec<String>,
    /// Maximum records per partition
    pub max_records_per_partition: Option<u64>,
    /// Maximum file size per partition (bytes)
    pub max_file_size_bytes: Option<u64>,
    /// Partition naming strategy
    pub naming_strategy: PartitionNamingStrategy,
}

/// Partition naming strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PartitionNamingStrategy {
    /// Use field values (e.g., year=2023/month=01/)
    FieldValues,
    /// Use sequential numbers (e.g., part-00001, part-00002)
    Sequential,
    /// Use timestamps (e.g., 2023-01-01-12-00-00)
    Timestamp,
    /// Custom naming pattern
    Custom(String),
}

impl Default for PartitionNamingStrategy {
    fn default() -> Self {
        Self::FieldValues
    }
}

/// Result of output generation
#[derive(Debug, Clone)]
pub struct OutputResult {
    /// Paths of generated output files
    pub output_paths: Vec<String>,
    /// Number of records written
    pub records_written: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Generation time in milliseconds
    pub generation_time_ms: u64,
    /// Output metadata
    pub metadata: HashMap<String, String>,
}

/// Statistics for output generation
#[derive(Debug, Clone, Default)]
pub struct OutputStats {
    /// Total number of output operations
    pub total_operations: u64,
    /// Total records written
    pub total_records_written: u64,
    /// Total bytes written
    pub total_bytes_written: u64,
    /// Total generation time in milliseconds
    pub total_generation_time_ms: u64,
    /// Number of successful operations
    pub successful_operations: u64,
    /// Number of failed operations
    pub failed_operations: u64,
}

impl OutputStats {
    /// Update statistics after an output operation
    pub fn update(&mut self, result: &OutputResult, success: bool) {
        self.total_operations += 1;
        self.total_generation_time_ms += result.generation_time_ms;
        
        if success {
            self.successful_operations += 1;
            self.total_records_written += result.records_written;
            self.total_bytes_written += result.bytes_written;
        } else {
            self.failed_operations += 1;
        }
    }

    /// Get success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.successful_operations as f64 / self.total_operations as f64
        }
    }

    /// Get average generation time per operation
    pub fn avg_generation_time_ms(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.total_generation_time_ms as f64 / self.total_operations as f64
        }
    }

    /// Get throughput (records per second)
    pub fn throughput_records_per_second(&self) -> f64 {
        if self.total_generation_time_ms == 0 {
            0.0
        } else {
            (self.total_records_written as f64) / (self.total_generation_time_ms as f64 / 1000.0)
        }
    }
}

/// Trait for format-specific output writers
#[async_trait]
pub trait FormatWriter: Send + Sync {
    /// Get the format name
    fn format_name(&self) -> &str;

    /// Get file extension for this format
    fn file_extension(&self) -> &str;

    /// Write series data
    async fn write_series(&mut self, series: &[Series], path: &Path, config: &OutputConfig) -> Result<OutputResult>;

    /// Write observation data
    async fn write_observations(&mut self, observations: &[Observation], path: &Path, config: &OutputConfig) -> Result<OutputResult>;

    /// Write lookup data
    async fn write_lookups(&mut self, lookups: &[Lookup], path: &Path, config: &OutputConfig) -> Result<OutputResult>;

    /// Write survey data
    async fn write_survey(&mut self, survey: &Survey, path: &Path, config: &OutputConfig) -> Result<OutputResult>;

    /// Write mixed data
    async fn write_mixed(&mut self, data: ProcessedData, path: &Path, config: &OutputConfig) -> Result<OutputResult>;

    /// Validate format-specific configuration
    fn validate_format_config(&self, config: &OutputConfig) -> Result<()>;

    /// Get format-specific default options
    fn default_format_options(&self) -> HashMap<String, String>;
}

/// Trait for output registry
pub trait OutputRegistry: Send + Sync {
    /// Register an output generator
    fn register_generator(&mut self, name: String, generator: Box<dyn OutputGenerator>) -> Result<()>;

    /// Unregister an output generator
    fn unregister_generator(&mut self, name: &str) -> Result<()>;

    /// Get an output generator by name
    fn get_generator(&self, name: &str) -> Result<&dyn OutputGenerator>;

    /// Get a mutable output generator by name
    fn get_generator_mut(&mut self, name: &str) -> Result<&mut dyn OutputGenerator>;

    /// List all registered generators
    fn list_generators(&self) -> Vec<String>;

    /// Check if a generator is registered
    fn has_generator(&self, name: &str) -> bool;

    /// Get generator for a specific format
    fn get_generator_for_format(&self, format: &str) -> Result<&dyn OutputGenerator>;

    /// Clear all registered generators
    fn clear(&mut self);
}

/// Trait for output factory
pub trait OutputFactory: Send + Sync {
    /// Create an output generator for the specified format
    fn create_generator(&self, format: &str, config: OutputConfig) -> Result<Box<dyn OutputGenerator>>;

    /// Create a format writer for the specified format
    fn create_writer(&self, format: &str) -> Result<Box<dyn FormatWriter>>;

    /// Get supported formats
    fn supported_formats(&self) -> Vec<String>;

    /// Check if a format is supported
    fn supports_format(&self, format: &str) -> bool {
        self.supported_formats().contains(&format.to_lowercase())
    }

    /// Get default configuration for a format
    fn default_config_for_format(&self, format: &str) -> Result<OutputConfig>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_config_default() {
        let config = OutputConfig::default();
        assert_eq!(config.format, "csv");
        assert_eq!(config.destination, "output");
        assert!(config.compression.is_none());
        assert!(config.partitioning.is_none());
    }

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert_eq!(config.algorithm, "gzip");
        assert_eq!(config.level, Some(6));
        assert!(!config.enabled);
    }

    #[test]
    fn test_partition_naming_strategy_default() {
        let strategy = PartitionNamingStrategy::default();
        matches!(strategy, PartitionNamingStrategy::FieldValues);
    }

    #[test]
    fn test_output_stats_update() {
        let mut stats = OutputStats::default();
        
        let result = OutputResult {
            output_paths: vec!["output.csv".to_string()],
            records_written: 1000,
            bytes_written: 50000,
            generation_time_ms: 1500,
            metadata: HashMap::new(),
        };
        
        stats.update(&result, true);
        
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.successful_operations, 1);
        assert_eq!(stats.total_records_written, 1000);
        assert_eq!(stats.total_bytes_written, 50000);
        assert_eq!(stats.success_rate(), 1.0);
    }

    #[test]
    fn test_output_stats_throughput() {
        let mut stats = OutputStats::default();
        
        let result = OutputResult {
            output_paths: vec!["output.csv".to_string()],
            records_written: 2000,
            bytes_written: 100000,
            generation_time_ms: 2000, // 2 seconds
            metadata: HashMap::new(),
        };
        
        stats.update(&result, true);
        
        // Should be 1000 records per second
        assert_eq!(stats.throughput_records_per_second(), 1000.0);
    }

    #[test]
    fn test_output_stats_failure() {
        let mut stats = OutputStats::default();
        
        let result = OutputResult {
            output_paths: vec![],
            records_written: 0,
            bytes_written: 0,
            generation_time_ms: 1000,
            metadata: HashMap::new(),
        };
        
        stats.update(&result, false);
        
        assert_eq!(stats.total_operations, 1);
        assert_eq!(stats.successful_operations, 0);
        assert_eq!(stats.failed_operations, 1);
        assert_eq!(stats.total_records_written, 0);
        assert_eq!(stats.success_rate(), 0.0);
    }
}