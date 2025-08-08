//! Reader factory implementation for BLS data processing
//!
//! This module provides a factory for creating appropriate reader instances
//! based on file characteristics, configuration, and performance requirements.
//! The factory automatically selects the optimal reader strategy.

use std::path::Path;
use std::collections::HashMap;
use std::sync::Arc;

use crate::data::reader::traits::{
    DataReader, SeriesReader, ObservationReader, LookupReader, SurveyReader,
    StreamingReader, MemoryMappedReader, ReaderFactory, ReaderConfig,
};
use crate::data::reader::file_reader::FileReader;
use crate::data::reader::mmap_reader::MmapReader;
use crate::error::types::{DataError, Result};

/// Default reader factory implementation
pub struct DefaultReaderFactory {
    /// Configuration for reader selection
    config: ReaderFactoryConfig,
    /// Cache of reader configurations by file type
    reader_configs: HashMap<String, ReaderConfig>,
}

/// Configuration for the reader factory
#[derive(Debug, Clone)]
pub struct ReaderFactoryConfig {
    /// Threshold file size for using memory mapping (in bytes)
    pub mmap_threshold: u64,
    /// Default buffer size for file readers (in bytes)
    pub default_buffer_size: usize,
    /// Default batch size for reading operations
    pub default_batch_size: usize,
    /// Whether to enable validation by default
    pub enable_validation: bool,
    /// Maximum number of errors to tolerate by default
    pub max_errors: usize,
    /// Whether to prefer memory mapping when available
    pub prefer_mmap: bool,
    /// File type specific configurations
    pub file_type_configs: HashMap<String, ReaderConfig>,
}

impl Default for ReaderFactoryConfig {
    fn default() -> Self {
        Self {
            mmap_threshold: 100 * 1024 * 1024, // 100MB
            default_buffer_size: 64 * 1024,    // 64KB
            default_batch_size: 1000,
            enable_validation: true,
            max_errors: 100,
            prefer_mmap: true,
            file_type_configs: HashMap::new(),
        }
    }
}

impl DefaultReaderFactory {
    /// Create a new reader factory with default configuration
    pub fn new() -> Self {
        Self::with_config(ReaderFactoryConfig::default())
    }

    /// Create a new reader factory with the given configuration
    pub fn with_config(config: ReaderFactoryConfig) -> Self {
        let mut reader_configs = HashMap::new();
        
        // Set up default configurations for different file types
        reader_configs.insert("series".to_string(), ReaderConfig {
            buffer_size: config.default_buffer_size,
            batch_size: config.default_batch_size,
            validate_on_read: config.enable_validation,
            use_memory_mapping: config.prefer_mmap,
            encoding: "UTF-8".to_string(),
            field_separator: '\t',
            skip_malformed: false,
            max_errors: config.max_errors,
        });

        reader_configs.insert("data".to_string(), ReaderConfig {
            buffer_size: config.default_buffer_size,
            batch_size: config.default_batch_size * 2, // Larger batch for data files
            validate_on_read: config.enable_validation,
            use_memory_mapping: config.prefer_mmap,
            encoding: "UTF-8".to_string(),
            field_separator: '\t',
            skip_malformed: true, // More lenient for data files
            max_errors: config.max_errors * 2,
        });

        reader_configs.insert("lookup".to_string(), ReaderConfig {
            buffer_size: config.default_buffer_size / 2, // Smaller buffer for lookup files
            batch_size: config.default_batch_size / 2,
            validate_on_read: config.enable_validation,
            use_memory_mapping: false, // Lookup files are usually small
            encoding: "UTF-8".to_string(),
            field_separator: '\t',
            skip_malformed: false,
            max_errors: config.max_errors / 2,
        });

        // Override with user-provided configurations
        for (file_type, user_config) in &config.file_type_configs {
            reader_configs.insert(file_type.clone(), user_config.clone());
        }

        Self {
            config,
            reader_configs,
        }
    }

    /// Determine the file type from the path
    fn determine_file_type(&self, path: &Path) -> Result<String> {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename.contains(".series") {
                return Ok("series".to_string());
            } else if filename.contains(".data") {
                return Ok("data".to_string());
            } else if filename.contains(".area") || filename.contains(".item") {
                return Ok("lookup".to_string());
            }
        }

        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            match extension {
                "series" => Ok("series".to_string()),
                "data" => Ok("data".to_string()),
                "area" | "item" => Ok("lookup".to_string()),
                "txt" => {
                    // Try to infer from filename
                    if let Some(filename) = path.file_stem().and_then(|n| n.to_str()) {
                        if filename.ends_with("series") {
                            Ok("series".to_string())
                        } else if filename.ends_with("data") {
                            Ok("data".to_string())
                        } else {
                            Ok("generic".to_string())
                        }
                    } else {
                        Ok("generic".to_string())
                    }
                }
                _ => Ok("generic".to_string()),
            }
        } else {
            Err(DataError::unsupported_format(
                format!("Cannot determine file type for: {}", path.display())
            ).into())
        }
    }

    /// Determine whether to use memory mapping for the given file
    fn should_use_mmap(&self, path: &Path, config: &ReaderConfig) -> Result<bool> {
        if !config.use_memory_mapping {
            return Ok(false);
        }

        if let Ok(metadata) = path.metadata() {
            let file_size = metadata.len();
            Ok(file_size >= self.config.mmap_threshold)
        } else {
            Ok(false)
        }
    }

    /// Get or create a reader configuration for the given file type
    fn get_reader_config(&self, file_type: &str) -> ReaderConfig {
        self.reader_configs.get(file_type)
            .cloned()
            .unwrap_or_else(|| {
                // Fallback to generic configuration
                ReaderConfig {
                    buffer_size: self.config.default_buffer_size,
                    batch_size: self.config.default_batch_size,
                    validate_on_read: self.config.enable_validation,
                    use_memory_mapping: self.config.prefer_mmap,
                    encoding: "UTF-8".to_string(),
                    field_separator: '\t',
                    skip_malformed: false,
                    max_errors: self.config.max_errors,
                }
            })
    }

    /// Create a reader optimized for the given file
    pub fn create_optimized_reader(&self, path: &Path) -> Result<Box<dyn DataReader>> {
        let file_type = self.determine_file_type(path)?;
        let config = self.get_reader_config(&file_type);
        
        if self.should_use_mmap(path, &config)? {
            Ok(Box::new(MmapReader::new(config)))
        } else {
            Ok(Box::new(FileReader::new(config)))
        }
    }

    /// Get supported file types
    pub fn get_supported_file_types(&self) -> Vec<String> {
        self.reader_configs.keys().cloned().collect()
    }

    /// Add or update a file type configuration
    pub fn set_file_type_config(&mut self, file_type: String, config: ReaderConfig) {
        self.reader_configs.insert(file_type, config);
    }

    /// Remove a file type configuration
    pub fn remove_file_type_config(&mut self, file_type: &str) {
        self.reader_configs.remove(file_type);
    }
}

impl ReaderFactory for DefaultReaderFactory {
    fn create_reader(&self, file_type: &str, config: ReaderConfig) -> Result<Box<dyn DataReader>> {
        // For explicit file type requests, use the provided configuration
        if config.use_memory_mapping {
            Ok(Box::new(MmapReader::new(config)))
        } else {
            Ok(Box::new(FileReader::new(config)))
        }
    }

    fn create_series_reader(&self, config: ReaderConfig) -> Result<Box<dyn SeriesReader>> {
        if config.use_memory_mapping {
            Ok(Box::new(MmapReader::new(config)))
        } else {
            Ok(Box::new(FileReader::new(config)))
        }
    }

    fn create_observation_reader(&self, config: ReaderConfig) -> Result<Box<dyn ObservationReader>> {
        // FileReader implements ObservationReader, MmapReader does not yet
        if config.use_memory_mapping {
            Err(DataError::not_implemented(
                "ObservationReader for MmapReader is not yet implemented".to_string()
            ).into())
        } else {
            Ok(Box::new(FileReader::new(config)))
        }
    }

    fn create_lookup_reader(&self, config: ReaderConfig) -> Result<Box<dyn LookupReader>> {
        // LookupReader would be implemented similarly to SeriesReader
        // For now, return an error indicating it's not yet implemented
        Err(DataError::not_implemented(
            "LookupReader implementation is not yet available".to_string()
        ).into())
    }

    fn create_survey_reader(&self, config: ReaderConfig) -> Result<Box<dyn SurveyReader>> {
        // SurveyReader would be implemented similarly to SeriesReader
        // For now, return an error indicating it's not yet implemented
        Err(DataError::not_implemented(
            "SurveyReader implementation is not yet available".to_string()
        ).into())
    }

    fn create_streaming_reader(&self, config: ReaderConfig) -> Result<Box<dyn StreamingReader>> {
        // StreamingReader would be implemented similarly to SeriesReader
        // For now, return an error indicating it's not yet implemented
        Err(DataError::not_implemented(
            "StreamingReader implementation is not yet available".to_string()
        ).into())
    }

    fn create_memory_mapped_reader(&self, config: ReaderConfig) -> Result<Box<dyn MemoryMappedReader>> {
        Ok(Box::new(MmapReader::new(config)))
    }

    fn supported_file_types(&self) -> Vec<String> {
        self.get_supported_file_types()
    }
}

/// Builder for creating reader factory configurations
pub struct ReaderFactoryConfigBuilder {
    config: ReaderFactoryConfig,
}

impl ReaderFactoryConfigBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: ReaderFactoryConfig::default(),
        }
    }

    /// Set the memory mapping threshold
    pub fn mmap_threshold(mut self, threshold: u64) -> Self {
        self.config.mmap_threshold = threshold;
        self
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

    /// Set the maximum number of errors to tolerate
    pub fn max_errors(mut self, max: usize) -> Self {
        self.config.max_errors = max;
        self
    }

    /// Set whether to prefer memory mapping
    pub fn prefer_mmap(mut self, prefer: bool) -> Self {
        self.config.prefer_mmap = prefer;
        self
    }

    /// Add a file type specific configuration
    pub fn file_type_config(mut self, file_type: String, config: ReaderConfig) -> Self {
        self.config.file_type_configs.insert(file_type, config);
        self
    }

    /// Build the configuration
    pub fn build(self) -> ReaderFactoryConfig {
        self.config
    }
}

impl Default for ReaderFactoryConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_factory_creation() {
        let factory = DefaultReaderFactory::new();
        let supported_types = factory.get_supported_file_types();
        assert!(supported_types.contains(&"series".to_string()));
        assert!(supported_types.contains(&"data".to_string()));
        assert!(supported_types.contains(&"lookup".to_string()));
    }

    #[test]
    fn test_file_type_determination() {
        let factory = DefaultReaderFactory::new();
        
        let series_path = Path::new("test.series");
        assert_eq!(factory.determine_file_type(series_path).unwrap(), "series");
        
        let data_path = Path::new("test.data.0");
        assert_eq!(factory.determine_file_type(data_path).unwrap(), "data");
        
        let area_path = Path::new("test.area");
        assert_eq!(factory.determine_file_type(area_path).unwrap(), "lookup");
    }

    #[test]
    fn test_mmap_threshold() {
        let config = ReaderFactoryConfigBuilder::new()
            .mmap_threshold(1024) // 1KB threshold for testing
            .build();
        
        let factory = DefaultReaderFactory::with_config(config);
        
        // Create a small file
        let mut small_file = NamedTempFile::with_suffix(".series").unwrap();
        small_file.write_all(b"small data").unwrap();
        
        let reader_config = factory.get_reader_config("series");
        assert!(!factory.should_use_mmap(small_file.path(), &reader_config).unwrap());
        
        // Create a larger file
        let mut large_file = NamedTempFile::with_suffix(".series").unwrap();
        let large_data = "x".repeat(2048); // 2KB
        large_file.write_all(large_data.as_bytes()).unwrap();
        
        assert!(factory.should_use_mmap(large_file.path(), &reader_config).unwrap());
    }

    #[test]
    fn test_config_builder() {
        let config = ReaderFactoryConfigBuilder::new()
            .mmap_threshold(50 * 1024 * 1024)
            .default_buffer_size(128 * 1024)
            .default_batch_size(2000)
            .enable_validation(false)
            .max_errors(500)
            .prefer_mmap(false)
            .build();
        
        assert_eq!(config.mmap_threshold, 50 * 1024 * 1024);
        assert_eq!(config.default_buffer_size, 128 * 1024);
        assert_eq!(config.default_batch_size, 2000);
        assert!(!config.enable_validation);
        assert_eq!(config.max_errors, 500);
        assert!(!config.prefer_mmap);
    }

    #[test]
    fn test_reader_creation() {
        let factory = DefaultReaderFactory::new();
        let config = ReaderConfig::default();
        
        // Test creating different reader types
        let reader = factory.create_reader("series", config.clone());
        assert!(reader.is_ok());
        
        let mmap_reader = factory.create_memory_mapped_reader(config);
        assert!(mmap_reader.is_ok());
    }
}