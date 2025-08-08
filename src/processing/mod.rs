//! # Processing Module
//!
//! This module provides comprehensive data processing capabilities for BLS survey data.
//! It implements a flexible, strategy-based architecture that can handle different
//! data sizes and processing requirements.
//!
//! ## Architecture
//!
//! The processing system is built around several key concepts:
//!
//! - **Processing Strategies**: Different approaches for handling data of various sizes
//!   - In-memory: For small datasets that fit entirely in memory
//!   - Chunked: For medium datasets processed in chunks
//!   - Memory-mapped: For large datasets using memory mapping
//!   - Streaming: For very large datasets processed as streams
//!
//! - **Pipeline Stages**: Modular processing steps that can be composed
//!   - Loader: Reads data from various sources
//!   - Transformer: Applies data transformations
//!   - Validator: Validates data integrity and business rules
//!   - Writer: Outputs processed data in various formats
//!
//! - **Registry System**: Dynamic registration and discovery of processors
//!
//! ## Usage
//!
//! ```rust
//! use rusty::processing::{ProcessingEngine, ProcessingConfig, ProcessingStrategy};
//! use rusty::error::Result;
//!
//! async fn process_survey_data() -> Result<()> {
//!     let config = ProcessingConfig::default()
//!         .strategy(ProcessingStrategy::InMemory)
//!         .with_max_threads(4);
//!     
//!     let mut engine = ProcessingEngine::new(config);
//!     engine.process_survey("AP").await?;
//!     Ok(())
//! }
//! ```

pub mod traits;
pub mod strategy;
pub mod pipeline;
pub mod registry;
pub mod engine;
pub mod dag;

// Re-export commonly used types and traits
pub use traits::{
    DataProcessor, ProcessingPipeline, ProcessorRegistry, ProcessorFactory,
    PipelineStage, LoaderStage, TransformerStage, ValidatorStage, WriterStage,
    ProcessingConfig, ProcessingStrategy, ProcessingInput, ProcessingOutput,
    ProcessingContext, ProcessingStats, ProcessedData,
    ValidationResult, ValidationRule, ValidationRuleType, ValidationSeverity,
    TransformationRule, TransformationRuleType,
};

pub use strategy::{
    InMemoryProcessor, ChunkedProcessor, MemoryMappedProcessor,
    create_processor, recommend_strategy,
};

pub use pipeline::{
    DefaultPipeline, LoaderStageImpl, TransformerStageImpl, 
    ValidatorStageImpl, WriterStageImpl,
};

pub use registry::{
    DefaultProcessorRegistry, ProcessorRegistryImpl,
};

pub use engine::{
    ProcessingEngine, ProcessingEngineBuilder, EngineStats, EngineStatus,
};

pub use dag::{
    DagExecutor, DagExecutionStats, DagExecutionContext, TaskState, TaskExecutionResult,
};

/// Create a new processing engine with default configuration
pub fn create_engine() -> ProcessingEngine {
    ProcessingEngine::new(ProcessingConfig::default())
}

/// Create a processing engine optimized for the given input
pub fn create_optimized_engine(input: &ProcessingInput) -> crate::error::Result<ProcessingEngine> {
    let strategy = recommend_strategy(input)?;
    let config = ProcessingConfig::default().with_strategy(strategy);
    Ok(ProcessingEngine::new(ProcessingConfig::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_engine() {
        let engine = create_engine();
        assert_eq!(engine.config().strategy, ProcessingStrategy::Auto);
    }

    #[test]
    fn test_module_exports() {
        // Test that all major types are accessible
        let _config = ProcessingConfig::default();
        let _input = ProcessingInput::new(vec!["test.data".to_string()]);
        let _output = ProcessingOutput::new(vec!["output.csv".to_string()], "csv".to_string());
    }
}