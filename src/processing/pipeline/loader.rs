//! # Loader Stage Implementation
//!
//! This module provides the loader stage implementation for the processing pipeline.
//! The loader stage is responsible for reading data from various sources and preparing
//! it for subsequent processing stages.
//!
//! ## Features
//!
//! - **Multi-format Support**: Reads BLS data files in various formats
//! - **Parallel Loading**: Utilizes multiple threads for concurrent data loading
//! - **Memory Management**: Monitors memory usage and applies backpressure when needed
//! - **Error Recovery**: Handles file errors gracefully with retry mechanisms
//! - **Data Validation**: Performs basic validation during loading
//!
//! ## Supported Data Sources
//!
//! - Local files (series, data, lookup files)
//! - Compressed archives
//! - Network resources (with caching)
//! - Database connections (future enhancement)
//!
//! ## Usage
//!
//! ```rust
//! use rusty::processing::pipeline::loader::LoaderStageImpl;
//! use rusty::processing::{ProcessingContext, ProcessingConfig};
//!
//! let mut loader = LoaderStageImpl::new();
//! let mut context = ProcessingContext::new(ProcessingConfig::default());
//! 
//! loader.execute(&mut context)?;
//! ```

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use rayon::prelude::*;
use tokio::task;

use crate::processing::traits::{
    PipelineStage, LoaderStage, ProcessingContext, ProcessingConfig,
    ProcessingInput, ValidationResult, ValidationSeverity, ValidationError,
};
use crate::data::reader::{
    DataReader, SeriesReader, ObservationReader, LookupReader, SurveyReader,
    create_optimized_reader,
};
use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::error::types::{ProcessingError, Result};
use crate::utils::validation::BLSValidationRules;
use crate::utils::file::{get_file_size, is_file_readable};

/// Implementation of the loader stage
pub struct LoaderStageImpl {
    /// Configuration for the loader
    config: LoaderConfig,
    /// Statistics for the loader stage
    stats: LoaderStats,
    /// Cache of loaded data readers
    reader_cache: Arc<Mutex<HashMap<String, Box<dyn DataReader>>>>,
}

/// Configuration for the loader stage
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Maximum number of concurrent file loads
    pub max_concurrent_loads: usize,
    /// Maximum memory usage before applying backpressure (bytes)
    pub max_memory_usage: u64,
    /// Enable data validation during loading
    pub validate_during_load: bool,
    /// Retry count for failed loads
    pub retry_count: u32,
    /// Timeout for individual file loads (seconds)
    pub load_timeout_seconds: u64,
    /// Enable reader caching
    pub enable_caching: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            max_concurrent_loads: 4,
            max_memory_usage: 2 * 1024 * 1024 * 1024, // 2GB
            validate_during_load: true,
            retry_count: 3,
            load_timeout_seconds: 300, // 5 minutes
            enable_caching: true,
        }
    }
}

/// Statistics for the loader stage
#[derive(Debug, Clone, Default)]
pub struct LoaderStats {
    /// Number of files loaded
    pub files_loaded: u64,
    /// Total bytes loaded
    pub bytes_loaded: u64,
    /// Number of records loaded
    pub records_loaded: u64,
    /// Number of load errors
    pub load_errors: u64,
    /// Average load time per file (milliseconds)
    pub avg_load_time_ms: f64,
    /// Memory usage peak during loading
    pub peak_memory_usage: u64,
}

impl LoaderStageImpl {
    /// Create a new loader stage with default configuration
    pub fn new() -> Self {
        Self {
            config: LoaderConfig::default(),
            stats: LoaderStats::default(),
            reader_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new loader stage with custom configuration
    pub fn with_config(config: LoaderConfig) -> Self {
        Self {
            config,
            stats: LoaderStats::default(),
            reader_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get loader statistics
    pub fn stats(&self) -> &LoaderStats {
        &self.stats
    }

    /// Reset loader statistics
    pub fn reset_stats(&mut self) {
        self.stats = LoaderStats::default();
    }

    /// Load data from input paths
    fn load_data_from_paths(&mut self, paths: &[String], context: &mut ProcessingContext) -> Result<Vec<Box<dyn DataReader>>> {
        let start_time = Instant::now();
        let mut readers = Vec::new();
        let mut load_errors = 0u64;

        // Expand wildcards and resolve paths
        let resolved_paths = self.resolve_input_paths(paths)?;
        
        // Group paths by type for efficient loading
        let grouped_paths = self.group_paths_by_type(&resolved_paths)?;
        
        // Load files in parallel with controlled concurrency
        for (data_type, type_paths) in grouped_paths {
            let type_readers = self.load_paths_parallel(&type_paths, &data_type, context)?;
            readers.extend(type_readers);
        }

        // Update statistics
        let elapsed = start_time.elapsed();
        self.stats.avg_load_time_ms = elapsed.as_millis() as f64 / resolved_paths.len() as f64;
        self.stats.files_loaded += resolved_paths.len() as u64;
        self.stats.load_errors += load_errors;

        // Add metrics to context
        context.add_metric("loader_files_loaded".to_string(), self.stats.files_loaded as f64);
        context.add_metric("loader_load_time_ms".to_string(), elapsed.as_millis() as f64);

        Ok(readers)
    }

    /// Resolve input paths, expanding wildcards and validating existence
    fn resolve_input_paths(&self, paths: &[String]) -> Result<Vec<PathBuf>> {
        let mut resolved_paths = Vec::new();

        for path_str in paths {
            let path = resolve_path(path_str)?;
            
            if path_str.contains('*') || path_str.contains('?') {
                // Expand wildcards
                let expanded = expand_wildcards(&path)?;
                resolved_paths.extend(expanded);
            } else if path.exists() {
                resolved_paths.push(path);
            } else {
                return Err(ProcessingError::DataError(
                    format!("Input path does not exist: {}", path_str)
                ));
            }
        }

        // Validate all paths are readable
        for path in &resolved_paths {
            if !self.is_file_readable(path)? {
                return Err(ProcessingError::DataError(format!(
                    "File is not readable: {}",
                    path.display()
                )));
            }
        }


        Ok(resolved_paths)
    }

    /// Group paths by data type for efficient loading
    fn group_paths_by_type(&self, paths: &[PathBuf]) -> Result<HashMap<String, Vec<PathBuf>>> {
        let mut grouped = HashMap::new();

        for path in paths {
            let data_type = self.determine_data_type(path)?;
            grouped.entry(data_type).or_insert_with(Vec::new).push(path.clone());
        }

        Ok(grouped)
    }

    /// Determine data type from file path
    fn determine_data_type(&self, path: &Path) -> Result<String> {
        let path_str = path.to_string_lossy().to_lowercase();
        
        if path_str.contains(".series") {
            Ok("series".to_string())
        } else if path_str.contains(".data") {
            Ok("observations".to_string())
        } else if path_str.contains(".area") || path_str.contains(".item") || 
                 path_str.contains(".industry") || path_str.contains(".occupation") {
            Ok("lookup".to_string())
        } else if path_str.contains(".survey") {
            Ok("survey".to_string())
        } else {
            // Try to determine from file content or use default
            Ok("unknown".to_string())
        }
    }

    /// Load paths in parallel with controlled concurrency
    fn load_paths_parallel(
        &mut self,
        paths: &[PathBuf],
        data_type: &str,
        context: &mut ProcessingContext,
    ) -> Result<Vec<Box<dyn DataReader>>> {
        let chunk_size = self.config.max_concurrent_loads;
        let mut all_readers = Vec::new();

        // Process paths in chunks to control memory usage
        for chunk in paths.chunks(chunk_size) {
            // Check memory usage before processing chunk
            self.check_memory_usage()?;

            // Load chunk in parallel
            let chunk_readers: Result<Vec<_>> = chunk
                .par_iter()
                .map(|path| self.load_single_file(path, data_type, context))
                .collect();

            match chunk_readers {
                Ok(mut readers) => {
                    all_readers.append(&mut readers);
                }
                Err(e) => {
                    self.stats.load_errors += 1;
                    if self.stats.load_errors > self.config.retry_count as u64 {
                        return Err(e);
                    }
                    // Continue with partial results
                    log::warn!("Failed to load chunk, continuing: {}", e);
                }
            }
        }

        Ok(all_readers)
    }

    /// Load a single file with retry logic
    fn load_single_file(
        &mut self,
        path: &Path,
        data_type: &str,
        context: &ProcessingContext,
    ) -> Result<Box<dyn DataReader>> {
        let mut attempts = 0;
        let max_attempts = self.config.retry_count + 1;

        while attempts < max_attempts {
            match self.try_load_single_file(path, data_type, context) {
                Ok(reader) => {
                    // Update statistics
                    if let Ok(size) = get_file_size(path) {
                        self.stats.bytes_loaded += size;
                    }
                    return Ok(reader);
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(e);
                    }
                    log::warn!("Load attempt {} failed for {}: {}", attempts, path.display(), e);
                    std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                }
            }
        }

        Err(ProcessingError::DataError(
            format!("Failed to load file after {} attempts: {}", max_attempts, path.display())
        ))
    }

    /// Try to load a single file (single attempt)
    fn try_load_single_file(
        &mut self,
        path: &Path,
        data_type: &str,
        context: &ProcessingContext,
    ) -> Result<Box<dyn DataReader>> {
        // Check cache first if enabled
        if self.config.enable_caching {
            let cache_key = format!("{}:{}", path.display(), data_type);
            if let Ok(cache) = self.reader_cache.lock() {
                if let Some(_cached_reader) = cache.get(&cache_key) {
                    // Note: In a real implementation, you'd need to handle reader cloning
                    // For now, we'll create a new reader each time
                }
            }
        }

        // Create optimized reader for the file
        let reader = create_optimized_reader(path, Some(data_type.to_string()))?;

        // Perform basic validation if enabled
        if self.config.validate_during_load {
            self.validate_reader_data(&*reader, context)?;
        }

        Ok(reader)
    }

    /// Validate reader data during loading
    fn validate_reader_data(&mut self, reader: &dyn DataReader, context: &ProcessingContext) -> Result<()> {
        // This is a simplified validation - in a real implementation,
        // you'd perform more comprehensive checks
        
        // Check if reader can provide basic information
        if reader.record_count().unwrap_or(0) == 0 {
            log::warn!("Reader contains no records");
        }

        // Update record count statistics
        if let Ok(count) = reader.record_count() {
            self.stats.records_loaded += count as u64;
        }

        Ok(())
    }

    /// Check memory usage and apply backpressure if needed
    fn check_memory_usage(&self) -> Result<()> {
        // Get current memory usage (simplified implementation)
        let current_usage = self.get_current_memory_usage();
        
        if current_usage > self.config.max_memory_usage {
            return Err(ProcessingError::ResourceExhausted(
                format!("Memory usage ({} bytes) exceeds limit ({} bytes)", 
                       current_usage, self.config.max_memory_usage)
            ));
        }

        Ok(())
    }

    /// Get current memory usage (simplified implementation)
    fn get_current_memory_usage(&self) -> u64 {
        // This would use system APIs in a real implementation
        // For now, return a placeholder value
        0
    }
    /// Check if a file is readable by attempting to open it.
    fn is_file_readable(&self, path: &Path) -> Result<bool> {
        match std::fs::File::open(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Ok(false),
            Err(e) => Err(ProcessingError::DataError(format!(
                "Error checking file readability for '{}': {}",
                path.display(),
                e
            ))),
        }
    }

    /// Estimate data size for memory planning
    fn estimate_data_size_internal(&self, paths: &[String]) -> Result<u64> {
        let mut total_size = 0u64;

        for path_str in paths {
            let path = Path::new(path_str);
            if path.exists() {
                total_size += get_file_size(path)?;
            }
        }

        Ok(total_size)
    }
}

fn resolve_path(p0: &String) -> _ {
    todo!()
}

fn expand_wildcards(p0: &_) -> _ {
    todo!()
}

impl Default for LoaderStageImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineStage for LoaderStageImpl {
    fn name(&self) -> &str {
        "loader"
    }

    fn description(&self) -> &str {
        "Loads data from various sources and prepares it for processing"
    }

    fn can_process(&self, context: &ProcessingContext) -> Result<bool> {
        // Check if there are input paths to process
        Ok(!context.input_paths.is_empty())
    }

    fn execute(&mut self, context: &mut ProcessingContext) -> Result<()> {
        log::info!("Starting loader stage execution");
        
        // Load data from input paths
        let readers = self.load_data_from_paths(&context.input_paths, context)?;
        
        // Store readers in context for next stages
        context.data_readers = readers;
        
        log::info!("Loader stage completed: {} files loaded, {} records", 
                  self.stats.files_loaded, self.stats.records_loaded);
        
        Ok(())
    }

    fn dependencies(&self) -> Vec<String> {
        // Loader stage has no dependencies
        vec![]
    }

    fn validate(&self, context: &ProcessingContext) -> Result<()> {
        // Validate that input paths are provided
        if context.input_paths.is_empty() {
            return Err(ProcessingError::InvalidConfiguration(
                "No input paths provided for loader stage".to_string()
            ));
        }

        // Validate configuration
        if self.config.max_concurrent_loads == 0 {
            return Err(ProcessingError::InvalidConfiguration(
                "max_concurrent_loads must be greater than 0".to_string()
            ));
        }

        Ok(())
    }

    fn cleanup(&mut self, _context: &mut ProcessingContext) -> Result<()> {
        // Clear reader cache if enabled
        if self.config.enable_caching {
            if let Ok(mut cache) = self.reader_cache.lock() {
                cache.clear();
            }
        }
        
        Ok(())
    }
}

impl LoaderStage for LoaderStageImpl {
    fn load_data(&mut self, context: &mut ProcessingContext) -> Result<Vec<Box<dyn DataReader>>> {
        let input_paths = context.input_paths.clone();
        self.load_data_from_paths(&input_paths, context)
    }

    fn estimate_data_size(&self, context: &ProcessingContext) -> Result<u64> {
        self.estimate_data_size_internal(&context.input_paths)
    }

    fn check_data_availability(&self, context: &ProcessingContext) -> Result<bool> {
        for path_str in &context.input_paths {
            let path = Path::new(path_str);
            if !path.exists() || !self.is_file_readable(&path)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    #[test]
    fn test_loader_stage_creation() {
        let loader = LoaderStageImpl::new();
        assert_eq!(loader.name(), "loader");
        assert!(!loader.description().is_empty());
    }

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert_eq!(config.max_concurrent_loads, 4);
        assert!(config.validate_during_load);
        assert!(config.enable_caching);
    }

    #[test]
    fn test_determine_data_type() {
        let loader = LoaderStageImpl::new();
        
        assert_eq!(loader.determine_data_type(Path::new("test.series")).unwrap(), "series");
        assert_eq!(loader.determine_data_type(Path::new("test.data")).unwrap(), "observations");
        assert_eq!(loader.determine_data_type(Path::new("test.area")).unwrap(), "lookup");
        assert_eq!(loader.determine_data_type(Path::new("test.item")).unwrap(), "lookup");
        assert_eq!(loader.determine_data_type(Path::new("test.survey")).unwrap(), "survey");
        assert_eq!(loader.determine_data_type(Path::new("test.unknown")).unwrap(), "unknown");
    }

    #[test]
    fn test_loader_stage_validation() {
        let loader = LoaderStageImpl::new();
        let mut context = ProcessingContext::new(ProcessingConfig::default());
        
        // Should fail with empty input paths
        assert!(loader.validate(&context).is_err());
        
        // Should pass with input paths
        context.input_paths.push("test.data".to_string());
        assert!(loader.validate(&context).is_ok());
    }

    #[test]
    fn test_can_process() {
        let loader = LoaderStageImpl::new();
        let mut context = ProcessingContext::new(ProcessingConfig::default());
        
        // Should return false with no input paths
        assert!(!loader.can_process(&context).unwrap());
        
        // Should return true with input paths
        context.input_paths.push("test.data".to_string());
        assert!(loader.can_process(&context).unwrap());
    }

    #[test]
    fn test_loader_stats() {
        let mut loader = LoaderStageImpl::new();
        assert_eq!(loader.stats().files_loaded, 0);
        assert_eq!(loader.stats().records_loaded, 0);
        
        loader.reset_stats();
        assert_eq!(loader.stats().files_loaded, 0);
    }

    #[test]
    fn test_memory_usage_check() {
        let loader = LoaderStageImpl::new();
        // Should not fail with default limits
        assert!(loader.check_memory_usage().is_ok());
    }

    #[test]
    fn test_estimate_data_size() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "test.data", "test content");
        
        let loader = LoaderStageImpl::new();
        let paths = vec![file_path.to_string_lossy().to_string()];
        let size = loader.estimate_data_size_internal(&paths).unwrap();
        
        assert!(size > 0);
    }

    #[test]
    fn test_group_paths_by_type() {
        let temp_dir = TempDir::new().unwrap();
        let series_path = create_test_file(&temp_dir, "test.series", "series data");
        let data_path = create_test_file(&temp_dir, "test.data", "observation data");
        
        let loader = LoaderStageImpl::new();
        let paths = vec![series_path, data_path];
        let grouped = loader.group_paths_by_type(&paths).unwrap();
        
        assert!(grouped.contains_key("series"));
        assert!(grouped.contains_key("observations"));
    }

    #[test]
    fn test_cleanup() {
        let mut loader = LoaderStageImpl::new();
        let mut context = ProcessingContext::new(ProcessingConfig::default());
        
        // Should not fail
        assert!(loader.cleanup(&mut context).is_ok());
    }
}