//! # Processing Pipeline Module
//!
//! This module provides a comprehensive pipeline system for processing BLS survey data.
//! The pipeline consists of modular stages that can be composed and configured to
//! handle different processing requirements.
//!
//! ## Pipeline Stages
//!
//! The processing pipeline consists of the following stages:
//!
//! 1. **Loader Stage**: Reads data from various sources (files, databases, APIs)
//! 2. **Transformer Stage**: Applies data transformations and business logic
//! 3. **Validator Stage**: Validates data integrity and business rules
//! 4. **Writer Stage**: Outputs processed data in various formats
//!
//! ## Pipeline Architecture
//!
//! The pipeline follows a chain-of-responsibility pattern where each stage:
//! - Receives a processing context
//! - Performs its specific operations
//! - Updates the context with results
//! - Passes control to the next stage
//!
//! ## Usage
//!
//! ```rust
//! use rusty::processing::pipeline::{DefaultPipeline, LoaderStageImpl};
//! use rusty::processing::{ProcessingConfig, ProcessingContext};
//!
//! let config = ProcessingConfig::default();
//! let mut pipeline = DefaultPipeline::new(config);
//! 
//! // Add custom stages
//! pipeline.add_stage(Box::new(LoaderStageImpl::new()))?;
//! 
//! // Execute pipeline
//! let mut context = ProcessingContext::new(config);
//! pipeline.execute(&mut context)?;
//! ```

pub mod loader;
pub mod transformer;
pub mod validator;
pub mod writer;
pub mod default;

use std::collections::HashMap;
use std::time::Instant;

use crate::processing::traits::{
    ProcessingPipeline, PipelineStage, ProcessingContext, ProcessingConfig,
    ValidationResult, ValidationSeverity,
};
use crate::error::types::{ProcessingError, Result};

// Re-export stage implementations
pub use loader::LoaderStageImpl;
pub use transformer::TransformerStageImpl;
pub use validator::ValidatorStageImpl;
pub use writer::WriterStageImpl;
pub use default::DefaultPipeline;

/// Pipeline execution statistics
#[derive(Debug, Clone)]
pub struct PipelineStats {
    /// Total execution time in milliseconds
    pub total_time_ms: u64,
    /// Time spent in each stage
    pub stage_times: HashMap<String, u64>,
    /// Number of records processed
    pub records_processed: u64,
    /// Number of errors encountered
    pub error_count: u64,
    /// Number of warnings generated
    pub warning_count: u64,
    /// Memory usage peak in bytes
    pub peak_memory_usage: u64,
}

impl Default for PipelineStats {
    fn default() -> Self {
        Self {
            total_time_ms: 0,
            stage_times: HashMap::new(),
            records_processed: 0,
            error_count: 0,
            warning_count: 0,
            peak_memory_usage: 0,
        }
    }
}

impl PipelineStats {
    /// Create new pipeline statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Add stage execution time
    pub fn add_stage_time(&mut self, stage_name: String, time_ms: u64) {
        self.stage_times.insert(stage_name, time_ms);
        self.total_time_ms += time_ms;
    }

    /// Update record count
    pub fn add_records(&mut self, count: u64) {
        self.records_processed += count;
    }

    /// Add validation results to statistics
    pub fn add_validation_results(&mut self, results: &[ValidationResult]) {
        for result in results {
            for error in &result.errors {
                match error.severity {
                    ValidationSeverity::Error | ValidationSeverity::Critical => {
                        self.error_count += 1;
                    }
                    ValidationSeverity::Warning => {
                        self.warning_count += 1;
                    }
                    ValidationSeverity::Info => {
                        // Info messages don't count as errors or warnings
                    }
                }
            }
        }
    }

    /// Update peak memory usage
    pub fn update_memory_usage(&mut self, current_usage: u64) {
        if current_usage > self.peak_memory_usage {
            self.peak_memory_usage = current_usage;
        }
    }

    /// Get processing rate (records per second)
    pub fn processing_rate(&self) -> f64 {
        if self.total_time_ms == 0 {
            0.0
        } else {
            (self.records_processed as f64) / (self.total_time_ms as f64 / 1000.0)
        }
    }

    /// Check if pipeline execution was successful
    pub fn is_successful(&self) -> bool {
        self.error_count == 0
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        format!(
            "Pipeline Stats: {} records in {}ms ({:.2} rec/sec), {} errors, {} warnings",
            self.records_processed,
            self.total_time_ms,
            self.processing_rate(),
            self.error_count,
            self.warning_count
        )
    }
}

/// Pipeline execution context with enhanced tracking
pub struct PipelineExecutionContext {
    /// Base processing context
    pub context: ProcessingContext,
    /// Pipeline statistics
    pub stats: PipelineStats,
    /// Stage execution order
    pub execution_order: Vec<String>,
    /// Current stage being executed
    pub current_stage: Option<String>,
    /// Start time of pipeline execution
    pub start_time: Instant,
}

impl PipelineExecutionContext {
    /// Create new pipeline execution context
    pub fn new(config: ProcessingConfig) -> Self {
        Self {
            context: ProcessingContext::new(config),
            stats: PipelineStats::new(),
            execution_order: Vec::new(),
            current_stage: None,
            start_time: Instant::now(),
        }
    }

    /// Start executing a stage
    pub fn start_stage(&mut self, stage_name: String) -> Instant {
        self.current_stage = Some(stage_name.clone());
        self.execution_order.push(stage_name);
        Instant::now()
    }

    /// Finish executing a stage
    pub fn finish_stage(&mut self, stage_name: String, start_time: Instant) {
        let elapsed = start_time.elapsed();
        self.stats.add_stage_time(stage_name, elapsed.as_millis() as u64);
        self.current_stage = None;
    }

    /// Add validation results
    pub fn add_validation_results(&mut self, results: &[ValidationResult]) {
        self.stats.add_validation_results(results);
    }

    /// Update memory usage
    pub fn update_memory_usage(&mut self) {
        // Get current memory usage (simplified implementation)
        let current_usage = get_current_memory_usage();
        self.stats.update_memory_usage(current_usage);
    }

    /// Finalize pipeline execution
    pub fn finalize(&mut self) {
        let total_elapsed = self.start_time.elapsed();
        self.stats.total_time_ms = total_elapsed.as_millis() as u64;
    }
}

/// Get current memory usage (simplified implementation)
fn get_current_memory_usage() -> u64 {
    // This is a simplified implementation
    // In a real system, you'd use system APIs to get actual memory usage
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb * 1024; // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    
    // Fallback: return 0 (unknown)
    0
}

/// Create a default pipeline with all standard stages
pub fn create_default_pipeline(config: ProcessingConfig) -> Result<DefaultPipeline> {
    let mut pipeline = DefaultPipeline::new(config);
    
    // Add standard stages in order
    pipeline.add_stage(Box::new(LoaderStageImpl::new()))?;
    pipeline.add_stage(Box::new(TransformerStageImpl::new()))?;
    pipeline.add_stage(Box::new(ValidatorStageImpl::new()))?;
    pipeline.add_stage(Box::new(WriterStageImpl::new()))?;
    
    Ok(pipeline)
}

/// Create a minimal pipeline with only loader and writer stages
pub fn create_minimal_pipeline(config: ProcessingConfig) -> Result<DefaultPipeline> {
    let mut pipeline = DefaultPipeline::new(config);
    
    // Add minimal stages
    pipeline.add_stage(Box::new(LoaderStageImpl::new()))?;
    pipeline.add_stage(Box::new(WriterStageImpl::new()))?;
    
    Ok(pipeline)
}

/// Create a validation-focused pipeline
pub fn create_validation_pipeline(config: ProcessingConfig) -> Result<DefaultPipeline> {
    let mut pipeline = DefaultPipeline::new(config);
    
    // Add stages focused on validation
    pipeline.add_stage(Box::new(LoaderStageImpl::new()))?;
    pipeline.add_stage(Box::new(ValidatorStageImpl::new()))?;
    
    Ok(pipeline)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_stats_creation() {
        let stats = PipelineStats::new();
        assert_eq!(stats.total_time_ms, 0);
        assert_eq!(stats.records_processed, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.warning_count, 0);
        assert!(stats.is_successful());
    }

    #[test]
    fn test_pipeline_stats_stage_time() {
        let mut stats = PipelineStats::new();
        stats.add_stage_time("loader".to_string(), 100);
        stats.add_stage_time("transformer".to_string(), 200);
        
        assert_eq!(stats.total_time_ms, 300);
        assert_eq!(stats.stage_times.get("loader"), Some(&100));
        assert_eq!(stats.stage_times.get("transformer"), Some(&200));
    }

    #[test]
    fn test_pipeline_stats_records() {
        let mut stats = PipelineStats::new();
        stats.add_records(1000);
        stats.add_records(500);
        
        assert_eq!(stats.records_processed, 1500);
    }

    #[test]
    fn test_pipeline_stats_processing_rate() {
        let mut stats = PipelineStats::new();
        stats.add_records(1000);
        stats.total_time_ms = 1000; // 1 second
        
        assert_eq!(stats.processing_rate(), 1000.0); // 1000 records per second
    }

    #[test]
    fn test_pipeline_stats_summary() {
        let mut stats = PipelineStats::new();
        stats.add_records(1000);
        stats.total_time_ms = 2000;
        stats.error_count = 5;
        stats.warning_count = 10;
        
        let summary = stats.summary();
        assert!(summary.contains("1000 records"));
        assert!(summary.contains("2000ms"));
        assert!(summary.contains("5 errors"));
        assert!(summary.contains("10 warnings"));
    }

    #[test]
    fn test_pipeline_execution_context() {
        let config = ProcessingConfig::default();
        let mut exec_context = PipelineExecutionContext::new(config);
        
        let start_time = exec_context.start_stage("test_stage".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        exec_context.finish_stage("test_stage".to_string(), start_time);
        
        assert_eq!(exec_context.execution_order.len(), 1);
        assert_eq!(exec_context.execution_order[0], "test_stage");
        assert!(exec_context.stats.stage_times.contains_key("test_stage"));
    }

    #[test]
    fn test_create_default_pipeline() {
        let config = ProcessingConfig::default();
        let pipeline = create_default_pipeline(config);
        assert!(pipeline.is_ok());
        
        let pipeline = pipeline.unwrap();
        assert_eq!(pipeline.get_stages().len(), 4); // loader, transformer, validator, writer
    }

    #[test]
    fn test_create_minimal_pipeline() {
        let config = ProcessingConfig::default();
        let pipeline = create_minimal_pipeline(config);
        assert!(pipeline.is_ok());
        
        let pipeline = pipeline.unwrap();
        assert_eq!(pipeline.get_stages().len(), 2); // loader, writer
    }

    #[test]
    fn test_create_validation_pipeline() {
        let config = ProcessingConfig::default();
        let pipeline = create_validation_pipeline(config);
        assert!(pipeline.is_ok());
        
        let pipeline = pipeline.unwrap();
        assert_eq!(pipeline.get_stages().len(), 2); // loader, validator
    }

    #[test]
    fn test_get_current_memory_usage() {
        let usage = get_current_memory_usage();
        // Should return 0 on non-Linux systems or if unable to read
        assert!(usage >= 0);
    }
}