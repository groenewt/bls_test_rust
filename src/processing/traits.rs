//! Processing trait definitions for BLS data processing
//!
//! This module defines the core traits for processing BLS data with different
//! strategies and pipeline stages. The traits are designed to be flexible,
//! performant, and support different processing approaches based on data size
//! and performance requirements.

use std::path::Path;
use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::data::reader::traits::{DataReader, ReaderConfig};
use crate::data::writer::traits::{DataWriter, WriterConfig};
use crate::error::types::{ProcessingError, Result};

/// Configuration for processing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Processing strategy to use
    pub strategy: ProcessingStrategy,
    /// Maximum number of threads to use
    pub max_threads: usize,
    /// Batch size for processing operations
    pub batch_size: usize,
    /// Buffer size for I/O operations (in bytes)
    pub buffer_size: usize,
    /// Whether to validate data during processing
    pub validate_data: bool,
    /// Maximum number of errors to tolerate before aborting
    pub max_errors: usize,
    /// Whether to continue processing on errors
    pub continue_on_error: bool,
    /// Memory limit for processing (in bytes, 0 = no limit)
    pub memory_limit: u64,
    /// Temporary directory for intermediate files
    pub temp_dir: Option<String>,
    /// Custom processing parameters
    pub custom_params: HashMap<String, String>,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            strategy: ProcessingStrategy::Auto,
            max_threads: num_cpus::get(),
            batch_size: 10000,
            buffer_size: 1024 * 1024, // 1MB
            validate_data: true,
            max_errors: 1000,
            continue_on_error: true,
            memory_limit: 0, // No limit
            temp_dir: None,
            custom_params: HashMap::new(),
        }
    }
}

/// Processing strategy enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProcessingStrategy {
    /// Automatically select strategy based on data size
    Auto,
    /// In-memory processing for small datasets
    InMemory,
    /// Chunked processing for medium datasets
    Chunked,
    /// Memory-mapped processing for large datasets
    MemoryMapped,
    /// Streaming processing for very large datasets
    Streaming,
}

/// Statistics collected during processing operations
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Total number of records processed
    pub records_processed: u64,
    /// Number of bytes processed
    pub bytes_processed: u64,
    /// Number of errors encountered
    pub errors_encountered: u64,
    /// Number of records skipped
    pub records_skipped: u64,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Memory usage peak in bytes
    pub peak_memory_usage: u64,
    /// Number of threads used
    pub threads_used: usize,
    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Processing context for sharing state between pipeline stages
#[derive(Debug, Clone)]
pub struct ProcessingContext {
    /// Processing configuration
    pub config: ProcessingConfig,
    /// Processing statistics
    pub stats: ProcessingStats,
    /// Input file paths
    pub input_paths: Vec<String>,
    /// Output file paths
    pub output_paths: Vec<String>,
    /// Temporary files created during processing
    pub temp_files: Vec<String>,
    /// Custom context data
    pub custom_data: HashMap<String, String>,
    pub data_readers: Vec<Box<dyn DataReader>>
}

impl ProcessingContext {
    /// Create a new processing context
    pub fn new(config: ProcessingConfig) -> Self {
        Self {
            config,
            stats: ProcessingStats::default(),
            input_paths: Vec::new(),
            output_paths: Vec::new(),
            temp_files: Vec::new(),
            custom_data: HashMap::new(),
            data_readers: vec![],
        }
    }

    /// Add a custom metric
    pub fn add_metric(&mut self, name: String, value: f64) {
        self.stats.custom_metrics.insert(name, value);
    }

    /// Get a custom metric
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        self.stats.custom_metrics.get(name).copied()
    }
}

/// Core trait for data processing
#[async_trait]
pub trait DataProcessor: Send + Sync {
    /// Get processor configuration
    fn config(&self) -> &ProcessingConfig;
    
    /// Get processing statistics
    fn stats(&self) -> &ProcessingStats;
    
    /// Reset processing statistics
    fn reset_stats(&mut self);
    
    /// Check if the processor can handle the given input
    fn can_process(&self, input: &ProcessingInput) -> Result<bool>;
    
    /// Process data with the given input and output specifications
    async fn process(&mut self, input: ProcessingInput, output: ProcessingOutput) -> Result<ProcessingContext>;
    
    /// Get supported processing strategies
    fn supported_strategies(&self) -> Vec<ProcessingStrategy>;
    
    /// Estimate memory usage for the given input
    fn estimate_memory_usage(&self, input: &ProcessingInput) -> Result<u64>;
    
    /// Validate processing configuration
    fn validate_config(&self, config: &ProcessingConfig) -> Result<()>;
}

/// Input specification for processing operations
#[derive(Debug, Clone)]
pub struct ProcessingInput {
    /// Input file paths
    pub paths: Vec<String>,
    /// Reader configuration
    pub reader_config: ReaderConfig,
    /// Input format hint
    pub format_hint: Option<String>,
    /// Custom input parameters
    pub custom_params: HashMap<String, String>,
    pub parameters: ()
}

impl ProcessingInput {
    /// Create a new processing input
    pub fn new(paths: Vec<String>) -> Self {
        Self {
            paths,
            reader_config: ReaderConfig::default(),
            format_hint: None,
            custom_params: HashMap::new(),
            parameters: (),
        }
    }

    /// Add a custom parameter
    pub fn with_param(mut self, key: String, value: String) -> Self {
        self.custom_params.insert(key, value);
        self
    }

    /// Set format hint
    pub fn with_format_hint(mut self, format: String) -> Self {
        self.format_hint = Some(format);
        self
    }
}

/// Output specification for processing operations
#[derive(Debug, Clone)]
pub struct ProcessingOutput {
    /// Output file paths
    pub paths: Vec<String>,
    /// Writer configuration
    pub writer_config: WriterConfig,
    /// Output format
    pub format: String,
    /// Custom output parameters
    pub custom_params: HashMap<String, String>,
}

impl ProcessingOutput {
    /// Create a new processing output
    pub fn new(paths: Vec<String>, format: String) -> Self {
        Self {
            paths,
            writer_config: WriterConfig::default(),
            format,
            custom_params: HashMap::new(),
        }
    }

    /// Add a custom parameter
    pub fn with_param(mut self, key: String, value: String) -> Self {
        self.custom_params.insert(key, value);
        self
    }
}

/// Trait for processing pipeline stages
#[async_trait]
pub trait PipelineStage: Send + Sync {
    /// Get stage name
    fn name(&self) -> &str;
    
    /// Get stage description
    fn description(&self) -> &str;
    
    /// Check if the stage can process the given context
    fn can_process(&self, context: &ProcessingContext) -> Result<bool>;
    
    /// Execute the pipeline stage
    async fn execute(&mut self, context: &mut ProcessingContext) -> Result<()>;
    
    /// Get stage dependencies (stages that must run before this one)
    fn dependencies(&self) -> Vec<String>;
    
    /// Validate stage configuration
    fn validate(&self, context: &ProcessingContext) -> Result<()>;
    
    /// Cleanup resources after stage execution
    async fn cleanup(&mut self, context: &mut ProcessingContext) -> Result<()>;
}

/// Trait for data loading stage
#[async_trait]
pub trait LoaderStage: PipelineStage {
    /// Load data from input sources
    async fn load_data(&mut self, context: &mut ProcessingContext) -> Result<Vec<Box<dyn DataReader>>>;
    
    /// Get estimated data size
    fn estimate_data_size(&self, context: &ProcessingContext) -> Result<u64>;
    
    /// Check data availability
    fn check_data_availability(&self, context: &ProcessingContext) -> Result<bool>;
}

/// Trait for data transformation stage
#[async_trait]
pub trait TransformerStage: PipelineStage {
    /// Transform series data
    async fn transform_series(&mut self, series: Vec<Series>, context: &mut ProcessingContext) -> Result<Vec<Series>>;
    
    /// Transform observation data
    async fn transform_observations(&mut self, observations: Vec<Observation>, context: &mut ProcessingContext) -> Result<Vec<Observation>>;
    
    /// Transform lookup data
    async fn transform_lookups(&mut self, lookups: Vec<Lookup>, context: &mut ProcessingContext) -> Result<Vec<Lookup>>;
    
    /// Transform survey data
    async fn transform_survey(&mut self, survey: Survey, context: &mut ProcessingContext) -> Result<Survey>;
    
    /// Get transformation rules
    fn get_transformation_rules(&self) -> Vec<TransformationRule>;
}

/// Trait for data validation stage
#[async_trait]
pub trait ValidatorStage: PipelineStage {
    /// Validate series data
    async fn validate_series(&mut self, series: &[Series], context: &mut ProcessingContext) -> Result<ValidationResult>;
    
    /// Validate observation data
    async fn validate_observations(&mut self, observations: &[Observation], context: &mut ProcessingContext) -> Result<ValidationResult>;
    
    /// Validate lookup data
    async fn validate_lookups(&mut self, lookups: &[Lookup], context: &mut ProcessingContext) -> Result<ValidationResult>;
    
    /// Validate survey data
    async fn validate_survey(&mut self, survey: &Survey, context: &mut ProcessingContext) -> Result<ValidationResult>;
    
    /// Get validation rules
    fn get_validation_rules(&self) -> Vec<ValidationRule>;
}

/// Trait for data writing stage
#[async_trait]
pub trait WriterStage: PipelineStage {
    /// Write processed data to output destinations
    async fn write_data(&mut self, data: ProcessedData, context: &mut ProcessingContext) -> Result<Vec<String>>;
    
    /// Get output writers
    async fn get_writers(&mut self, context: &ProcessingContext) -> Result<Vec<Box<dyn DataWriter>>>;
    
    /// Finalize output files
    async fn finalize_output(&mut self, context: &mut ProcessingContext) -> Result<()>;
}

/// Processed data container
#[derive(Debug, Clone)]
pub enum ProcessedData {
    Series(Vec<Series>),
    Observations(Vec<Observation>),
    Lookups(Vec<Lookup>),
    Survey(Survey),
    Mixed {
        series: Vec<Series>,
        observations: Vec<Observation>,
        lookups: Vec<Lookup>,
        surveys: Vec<Survey>,
    },
}

/// Transformation rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule type
    pub rule_type: TransformationRuleType,
    /// Rule parameters
    pub parameters: HashMap<String, String>,
    /// Whether the rule is enabled
    pub enabled: bool,
    pub target_field: Option<String>,
    pub source_field: Option<String>,
    pub priority: i32,
}

/// Transformation rule types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformationRuleType {
    /// Field mapping rule
    FieldMapping,
    /// Data type conversion rule
    TypeConversion,
    /// Value transformation rule
    ValueTransformation,
    /// Aggregation rule
    Aggregation,
    /// Filtering rule
    Filtering,
    /// Custom transformation rule
    Custom(String),
}

/// Validation rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Rule type
    pub rule_type: ValidationRuleType,
    /// Rule parameters
    pub parameters: HashMap<String, String>,
    /// Severity level
    pub severity: ValidationSeverity,
    /// Whether the rule is enabled
    pub enabled: bool,
}

/// Validation rule types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRuleType {
    /// Required field validation
    RequiredField,
    /// Data type validation
    DataType,
    /// Range validation
    Range,
    /// Format validation
    Format,
    /// Uniqueness validation
    Uniqueness,
    /// Cross-field validation
    CrossField,
    /// Custom validation rule
    Custom(String),
}

/// Validation severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Information level
    Info,
    /// Warning level
    Warning,
    /// Error level
    Error,
    /// Critical error level
    Critical,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub passed: bool,
    /// Number of records validated
    pub records_validated: u64,
    /// Validation errors
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
    /// Custom validation metrics
    pub metrics: HashMap<String, f64>,
    pub is_valid: bool,
}

/// Validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error message
    pub message: String,
    /// Rule that failed
    pub rule: String,
    /// Record index (if applicable)
    pub record_index: Option<u64>,
    /// Field name (if applicable)
    pub field_name: Option<String>,
    /// Error severity
    pub severity: ValidationSeverity,
}

/// Validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,
    /// Rule that generated the warning
    pub rule: String,
    /// Record index (if applicable)
    pub record_index: Option<u64>,
    /// Field name (if applicable)
    pub field_name: Option<String>,
}

/// Trait for processing pipeline
#[async_trait]
pub trait ProcessingPipeline: Send + Sync {
    /// Add a stage to the pipeline
    fn add_stage(&mut self, stage: Box<dyn PipelineStage>) -> Result<()>;
    
    /// Remove a stage from the pipeline
    fn remove_stage(&mut self, stage_name: &str) -> Result<()>;
    
    /// Get pipeline stages
    fn get_stages(&self) -> Vec<&dyn PipelineStage>;
    
    /// Execute the entire pipeline
    async fn execute(&mut self, context: &mut ProcessingContext) -> Result<()>;
    
    /// Validate pipeline configuration
    fn validate(&self, context: &ProcessingContext) -> Result<()>;
    
    /// Get pipeline execution plan
    fn get_execution_plan(&self) -> Result<Vec<String>>;
}

/// Trait for processor registry
pub trait ProcessorRegistry: Send + Sync {
    /// Register a processor
    fn register_processor(&mut self, name: String, processor: Box<dyn DataProcessor>) -> Result<()>;
    
    /// Unregister a processor
    fn unregister_processor(&mut self, name: &str) -> Result<()>;
    
    /// Get a processor by name
    fn get_processor(&self, name: &str) -> Result<&dyn DataProcessor>;
    
    /// Get a mutable processor by name
    fn get_processor_mut(&mut self, name: &str) -> Result<&mut dyn DataProcessor>;
    
    /// List all registered processors
    fn list_processors(&self) -> Vec<String>;
    
    /// Check if a processor is registered
    fn has_processor(&self, name: &str) -> bool;
    
    /// Clear all registered processors
    fn clear(&mut self);
}

/// Factory trait for creating processors
pub trait ProcessorFactory: Send + Sync {
    /// Create a processor for the given strategy
    fn create_processor(&self, strategy: ProcessingStrategy, config: ProcessingConfig) -> Result<Box<dyn DataProcessor>>;
    
    /// Create a pipeline with default stages
    fn create_pipeline(&self, config: ProcessingConfig) -> Result<Box<dyn ProcessingPipeline>>;
    
    /// Get supported strategies
    fn supported_strategies(&self) -> Vec<ProcessingStrategy>;
    
    /// Get recommended strategy for the given input
    fn recommend_strategy(&self, input: &ProcessingInput) -> Result<ProcessingStrategy>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processing_config_default() {
        let config = ProcessingConfig::default();
        assert_eq!(config.strategy, ProcessingStrategy::Auto);
        assert!(config.max_threads > 0);
        assert!(config.batch_size > 0);
        assert!(config.validate_data);
    }

    #[test]
    fn test_processing_input_creation() {
        let input = ProcessingInput::new(vec!["test.csv".to_string()])
            .with_param("key".to_string(), "value".to_string())
            .with_format_hint("csv".to_string());
        
        assert_eq!(input.paths.len(), 1);
        assert_eq!(input.custom_params.get("key"), Some(&"value".to_string()));
        assert_eq!(input.format_hint, Some("csv".to_string()));
    }

    #[test]
    fn test_processing_output_creation() {
        let output = ProcessingOutput::new(vec!["output.parquet".to_string()], "parquet".to_string())
            .with_param("compression".to_string(), "snappy".to_string());
        
        assert_eq!(output.paths.len(), 1);
        assert_eq!(output.format, "parquet");
        assert_eq!(output.custom_params.get("compression"), Some(&"snappy".to_string()));
    }

    #[test]
    fn test_processing_context() {
        let config = ProcessingConfig::default();
        let mut context = ProcessingContext::new(config);
        
        context.add_metric("test_metric".to_string(), 42.0);
        assert_eq!(context.get_metric("test_metric"), Some(42.0));
        assert_eq!(context.get_metric("nonexistent"), None);
    }

    #[test]
    fn test_validation_severity_ordering() {
        assert!(ValidationSeverity::Critical > ValidationSeverity::Error);
        assert!(ValidationSeverity::Error > ValidationSeverity::Warning);
        assert!(ValidationSeverity::Warning > ValidationSeverity::Info);
    }
}