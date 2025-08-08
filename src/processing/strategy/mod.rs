//! # Processing Strategy Module
//!
//! This module contains different processing strategies for handling BLS survey data
//! of various sizes and characteristics. Each strategy is optimized for specific
//! use cases and data volumes.
//!
//! ## Available Strategies
//!
//! - **InMemoryProcessor**: Loads all data into memory for fast processing.
//!   Best for small to medium datasets (< 1GB).
//!
//! - **ChunkedProcessor**: Processes data in configurable chunks.
//!   Best for medium to large datasets (1GB - 10GB).
//!
//! - **MemoryMappedProcessor**: Uses memory mapping for efficient large file access.
//!   Best for very large datasets (> 10GB) with random access patterns.
//!
//! ## Strategy Selection
//!
//! The system can automatically recommend the best strategy based on:
//! - Input data size
//! - Available system memory
//! - Processing requirements
//! - Performance characteristics
//!
//! ## Usage
//!
//! ```rust
//! use rusty::processing::strategy::{create_processor, recommend_strategy};
//! use rusty::processing::{ProcessingConfig, ProcessingInput, ProcessingStrategy};
//!
//! // Automatic strategy selection
//! let input = ProcessingInput::new(vec!["large_dataset.data".to_string()]);
//! let strategy = recommend_strategy(&input)?;
//! let processor = create_processor(strategy, ProcessingConfig::default())?;
//!
//! // Manual strategy selection
//! let processor = create_processor(
//!     ProcessingStrategy::InMemory,
//!     ProcessingConfig::default()
//! )?;
//! ```

pub mod in_memory;
pub mod chunked;
pub mod mmap;
pub mod factory;

use std::path::Path;
use crate::processing::traits::{
    DataProcessor, ProcessingStrategy, ProcessingConfig, ProcessingInput
};
use crate::error::types::{ProcessingError, Result};

// Re-export strategy implementations
pub use in_memory::InMemoryProcessor;
pub use chunked::ChunkedProcessor;
pub use mmap::MemoryMappedProcessor;
pub use factory::ProcessorFactory;

/// Create a processor instance for the specified strategy
pub fn create_processor(
    strategy: ProcessingStrategy,
    config: ProcessingConfig,
) -> Result<Box<dyn DataProcessor>> {
    match strategy {
        ProcessingStrategy::InMemory => {
            Ok(Box::new(InMemoryProcessor::new(config)))
        }
        ProcessingStrategy::Chunked => {
            Ok(Box::new(ChunkedProcessor::new(config)))
        }
        ProcessingStrategy::MemoryMapped => {
            Ok(Box::new(MemoryMappedProcessor::new(config)))
        }
        ProcessingStrategy::Auto => {
            // For auto strategy, we need input to make a recommendation
            Err(ProcessingError::invalid_configuration(
                "Auto strategy requires input analysis. Use recommend_strategy() first.".to_string()
            ))
        }
        ProcessingStrategy::Streaming => {
            // Streaming strategy not yet implemented
            Err(ProcessingError::invalid_configuration(
                "Streaming strategy is not yet implemented".to_string()
            ))
        }
    }
}

/// Recommend the best processing strategy for the given input
pub fn recommend_strategy(input: &ProcessingInput) -> Result<ProcessingStrategy> {
    let total_size = estimate_total_input_size(input)?;
    let available_memory = get_available_memory();
    
    // Strategy selection logic based on data size and available memory
    if total_size < 100 * 1024 * 1024 { // < 100MB
        Ok(ProcessingStrategy::InMemory)
    } else if total_size < 1024 * 1024 * 1024 { // < 1GB
        if available_memory > total_size * 3 { // 3x safety margin
            Ok(ProcessingStrategy::InMemory)
        } else {
            Ok(ProcessingStrategy::Chunked)
        }
    } else if total_size < 10 * 1024 * 1024 * 1024 { // < 10GB
        Ok(ProcessingStrategy::Chunked)
    } else {
        Ok(ProcessingStrategy::MemoryMapped)
    }
}

/// Estimate the total size of input data
fn estimate_total_input_size(input: &ProcessingInput) -> Result<u64> {
    let mut total_size = 0u64;
    
    for path_str in &input.paths {
        let path = Path::new(path_str);
        if path.exists() {
            if path.is_file() {
                total_size += path.metadata()
                    .map_err(|e| ProcessingError::io_error(format!("Failed to get file metadata: {}", e)))?
                    .len();
            } else if path.is_dir() {
                total_size += estimate_directory_size(path)?;
            }
        } else {
            // Path doesn't exist, estimate based on typical BLS file sizes
            total_size += estimate_bls_file_size(path_str);
        }
    }
    
    Ok(total_size)
}

/// Estimate the size of all files in a directory
fn estimate_directory_size(dir: &Path) -> Result<u64> {
    let mut total_size = 0u64;
    
    let entries = std::fs::read_dir(dir)
        .map_err(|e| ProcessingError::io_error(format!("Failed to read directory: {}", e)))?;
    
    for entry in entries {
        let entry = entry
            .map_err(|e| ProcessingError::io_error(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        
        if path.is_file() {
            total_size += entry.metadata()
                .map_err(|e| ProcessingError::io_error(format!("Failed to get file metadata: {}", e)))?
                .len();
        } else if path.is_dir() {
            total_size += estimate_directory_size(&path)?;
        }
    }
    
    Ok(total_size)
}

/// Estimate BLS file size based on survey type and file extension
fn estimate_bls_file_size(path: &str) -> u64 {
    let path_lower = path.to_lowercase();
    
    // Estimate based on file type
    if path_lower.contains(".series") {
        50 * 1024 * 1024 // 50MB typical for series files
    } else if path_lower.contains(".data") {
        500 * 1024 * 1024 // 500MB typical for data files
    } else if path_lower.contains(".area") || path_lower.contains(".item") {
        10 * 1024 * 1024 // 10MB typical for lookup files
    } else {
        100 * 1024 * 1024 // 100MB default estimate
    }
}

/// Get available system memory in bytes
fn get_available_memory() -> u64 {
    // This is a simplified implementation
    // In a real system, you'd use system APIs to get actual available memory
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: assume 8GB available memory
    8 * 1024 * 1024 * 1024
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::traits::ProcessingInput;

    #[test]
    fn test_create_in_memory_processor() {
        let config = ProcessingConfig::default();
        let processor = create_processor(ProcessingStrategy::InMemory, config);
        assert!(processor.is_ok());
    }

    #[test]
    fn test_recommend_strategy_small_data() {
        let input = ProcessingInput::new(vec!["small_file.data".to_string()]);
        let strategy = recommend_strategy(&input);
        assert!(strategy.is_ok());
        // For non-existent files, it should still recommend a strategy
    }

    #[test]
    fn test_auto_strategy_error() {
        let config = ProcessingConfig::default();
        let result = create_processor(ProcessingStrategy::Auto, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_strategy_not_implemented() {
        let config = ProcessingConfig::default();
        let result = create_processor(ProcessingStrategy::Streaming, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_bls_file_size_estimation() {
        assert_eq!(estimate_bls_file_size("test.series"), 50 * 1024 * 1024);
        assert_eq!(estimate_bls_file_size("test.data"), 500 * 1024 * 1024);
        assert_eq!(estimate_bls_file_size("test.area"), 10 * 1024 * 1024);
        assert_eq!(estimate_bls_file_size("unknown.ext"), 100 * 1024 * 1024);
    }

    #[test]
    fn test_get_available_memory() {
        let memory = get_available_memory();
        assert!(memory > 0);
    }
}