//! # Processing Engine Implementation
//!
//! This module provides the main processing engine that orchestrates the entire
//! data processing workflow.

use std::time::Instant;
use std::sync::Arc;
use crate::config::Config;
use crate::processing::traits::{
    ProcessingPipeline, ProcessorFactory, ProcessingConfig, ProcessingInput,
    ProcessingOutput, ProcessingContext, ProcessingStrategy,
};
use crate::processing::pipeline::{create_default_pipeline, DefaultPipeline};
use crate::processing::strategy::factory::DefaultProcessorFactory;
use crate::processing::registry::{ProcessorRegistryImpl, RegistryConfig};
use crate::error::types::{ProcessingError, Result};

/// Main processing engine that orchestrates the entire workflow
pub struct ProcessingEngine {
    /// Engine configuration
    config: ProcessingConfig,
    /// Processing pipeline
    pipeline: Box<dyn ProcessingPipeline>,
    /// Processor factory
    factory: Arc<DefaultProcessorFactory>,
    /// Processor registry
    registry: Arc<ProcessorRegistryImpl>,
    /// Engine statistics
    stats: EngineStats,
}

/// Statistics for the processing engine
#[derive(Debug, Clone, Default)]
pub struct EngineStats {
    /// Total number of processing runs
    pub total_runs: u64,
    /// Total processing time in milliseconds
    pub total_processing_time_ms: u64,
    /// Number of successful runs
    pub successful_runs: u64,
    /// Number of failed runs
    pub failed_runs: u64,
    /// Average processing time per run
    pub avg_processing_time_ms: f64,
}

impl EngineStats {
    /// Update statistics after a processing run
    pub fn update(&mut self, processing_time_ms: u64, success: bool) {
        self.total_runs += 1;
        self.total_processing_time_ms += processing_time_ms;
        
        if success {
            self.successful_runs += 1;
        } else {
            self.failed_runs += 1;
        }
        
        self.avg_processing_time_ms = self.total_processing_time_ms as f64 / self.total_runs as f64;
    }

    /// Get success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_runs == 0 {
            0.0
        } else {
            self.successful_runs as f64 / self.total_runs as f64
        }
    }
}

impl ProcessingEngine {
    /// Create a new processing engine with the given configuration
    pub fn new(config: Config) -> Self {
        let factory = Arc::new(DefaultProcessorFactory::new());
        let registry = Arc::new(ProcessorRegistryImpl::default());
        let pipeline = Box::new(DefaultPipeline::new(config.clone()));

        Self {
            config,
            pipeline,
            factory,
            registry,
            stats: EngineStats::default(),
        }
    }

    /// Create a processing engine with a custom pipeline
    pub fn with_pipeline(config: ProcessingConfig, pipeline: Box<dyn ProcessingPipeline>) -> Self {
        let factory = Arc::new(DefaultProcessorFactory::new());
        let registry = Arc::new(ProcessorRegistryImpl::default());

        Self {
            config,
            pipeline,
            factory,
            registry,
            stats: EngineStats::default(),
        }
    }

    /// Get engine configuration
    pub fn config(&self) -> &ProcessingConfig {
        &self.config
    }

    /// Get engine statistics
    pub fn stats(&self) -> &EngineStats {
        &self.stats
    }

    /// Get processor factory
    pub fn factory(&self) -> Arc<DefaultProcessorFactory> {
        Arc::clone(&self.factory)
    }

    /// Get processor registry
    pub fn registry(&self) -> Arc<ProcessorRegistryImpl> {
        Arc::clone(&self.registry)
    }

    /// Process data with the given input and output specifications
    pub async fn process(&mut self, input: ProcessingInput, output: ProcessingOutput) -> Result<ProcessingContext> {
        let start_time = Instant::now();
        
        log::info!("Starting processing engine with {} input paths", input.paths.len());
        
        // Create processing context
        let mut context = ProcessingContext::new(self.config.clone());
        context.input_paths = input.paths.clone();
        context.output_paths = output.paths.clone();
        
        // Add input parameters to context
        for (key, value) in input.parameters {
            context.add_metric(key, value.parse().unwrap_or(0.0));
        }
        
        // Execute the pipeline
        let result = self.pipeline.execute(&mut context);
        
        let elapsed = start_time.elapsed();
        let processing_time_ms = elapsed.as_millis() as u64;
        
        match result {
            Ok(()) => {
                self.stats.update(processing_time_ms, true);
                log::info!("Processing completed successfully in {}ms", processing_time_ms);
                Ok(context)
            }
            Err(e) => {
                self.stats.update(processing_time_ms, false);
                log::error!("Processing failed after {}ms: {}", processing_time_ms, e);
                Err(e)
            }
        }
    }

    /// Process a survey by name
    pub async fn process_survey(&mut self, survey_code: &str) -> Result<ProcessingContext> {
        log::info!("Processing survey: {}", survey_code);
        
        // Create input specification for the survey
        let input = ProcessingInput::new(vec![
            format!("data/raw/bls/{}/{}.series", survey_code, survey_code),
            format!("data/raw/bls/{}/{}.data", survey_code, survey_code),
        ]);
        
        // Create output specification
        let output = ProcessingOutput::new(
            vec![format!("data/processed/{}/output.csv", survey_code)],
            "csv".to_string(),
        );
        
        self.process(input, output).await
    }

    /// Validate the engine configuration and pipeline
    pub fn validate(&self) -> Result<()> {
        // Validate configuration
        if self.config.max_threads == 0 {
            return Err(ProcessingError::InvalidConfiguration(
                "max_threads must be greater than 0".to_string()
            ));
        }
        
        // Validate pipeline
        let context = ProcessingContext::new(self.config.clone());
        self.pipeline.validate(&context)?;
        
        log::info!("Engine validation passed");
        Ok(())
    }

    /// Get execution plan from the pipeline
    pub fn get_execution_plan(&self) -> Result<Vec<String>> {
        self.pipeline.get_execution_plan()
    }

    /// Reset engine statistics
    pub fn reset_stats(&mut self) {
        self.stats = EngineStats::default();
    }

    /// Get detailed engine status
    pub fn get_status(&self) -> EngineStatus {
        EngineStatus {
            config: self.config.clone(),
            stats: self.stats.clone(),
            pipeline_stages: self.pipeline.get_stages().len(),
            registry_processors: self.registry.get_statistics().map(|s| s.total_processors).unwrap_or(0),
        }
    }
}

/// Detailed status information for the processing engine
#[derive(Debug, Clone)]
pub struct EngineStatus {
    /// Engine configuration
    pub config: ProcessingConfig,
    /// Engine statistics
    pub stats: EngineStats,
    /// Number of pipeline stages
    pub pipeline_stages: usize,
    /// Number of registered processors
    pub registry_processors: usize,
}

/// Builder for creating processing engines with custom configurations
pub struct ProcessingEngineBuilder {
    config: ProcessingConfig,
    pipeline: Option<Box<dyn ProcessingPipeline>>,
    factory: Option<Arc<DefaultProcessorFactory>>,
    registry: Option<Arc<ProcessorRegistryImpl>>,
}

impl ProcessingEngineBuilder {
    /// Create a new engine builder
    pub fn new() -> Self {
        Self {
            config: ProcessingConfig::default(),
            pipeline: None,
            factory: None,
            registry: None,
        }
    }

    /// Set the processing configuration
    pub fn with_config(mut self, config: ProcessingConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the processing strategy
    pub fn with_strategy(mut self, strategy: ProcessingStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }

    /// Set the maximum number of threads
    pub fn with_max_threads(mut self, max_threads: usize) -> Self {
        self.config.max_threads = max_threads;
        self
    }

    /// Set a custom pipeline
    pub fn with_pipeline(mut self, pipeline: Box<dyn ProcessingPipeline>) -> Self {
        self.pipeline = Some(pipeline);
        self
    }

    /// Set a custom processor factory
    pub fn with_factory(mut self, factory: Arc<DefaultProcessorFactory>) -> Self {
        self.factory = Some(factory);
        self
    }

    /// Set a custom processor registry
    pub fn with_registry(mut self, registry: Arc<ProcessorRegistryImpl>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Build the processing engine
    pub fn build(self) -> Result<ProcessingEngine> {
        let factory = self.factory.unwrap_or_else(|| Arc::new(DefaultProcessorFactory::new()));
        let registry = self.registry.unwrap_or_else(|| Arc::new(ProcessorRegistryImpl::default()));
        
        let pipeline = if let Some(pipeline) = self.pipeline {
            pipeline
        } else {
            Box::new(create_default_pipeline(self.config.clone())?)
        };

        let engine = ProcessingEngine {
            config: self.config,
            pipeline,
            factory,
            registry,
            stats: EngineStats::default(),
        };

        // Validate the engine before returning
        engine.validate()?;

        Ok(engine)
    }
}

impl Default for ProcessingEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let config = ProcessingConfig::default();
        let engine = ProcessingEngine::new(config);
        
        assert_eq!(engine.stats().total_runs, 0);
        assert_eq!(engine.stats().success_rate(), 0.0);
    }

    #[test]
    fn test_engine_stats_update() {
        let mut stats = EngineStats::default();
        
        stats.update(1000, true);
        assert_eq!(stats.total_runs, 1);
        assert_eq!(stats.successful_runs, 1);
        assert_eq!(stats.failed_runs, 0);
        assert_eq!(stats.success_rate(), 1.0);
        
        stats.update(2000, false);
        assert_eq!(stats.total_runs, 2);
        assert_eq!(stats.successful_runs, 1);
        assert_eq!(stats.failed_runs, 1);
        assert_eq!(stats.success_rate(), 0.5);
    }

    #[test]
    fn test_engine_validation() {
        let config = ProcessingConfig::default();
        let engine = ProcessingEngine::new(config);
        
        assert!(engine.validate().is_ok());
    }

    #[test]
    fn test_engine_validation_invalid_config() {
        let mut config = ProcessingConfig::default();
        config.max_threads = 0;
        let engine = ProcessingEngine::new(config);
        
        assert!(engine.validate().is_err());
    }

    #[test]
    fn test_engine_builder() {
        let builder = ProcessingEngineBuilder::new()
            .with_strategy(ProcessingStrategy::InMemory)
            .with_max_threads(8);
        
        let engine = builder.build();
        assert!(engine.is_ok());
        
        let engine = engine.unwrap();
        assert_eq!(engine.config().strategy, ProcessingStrategy::InMemory);
        assert_eq!(engine.config().max_threads, 8);
    }

    #[test]
    fn test_engine_builder_invalid_config() {
        let builder = ProcessingEngineBuilder::new()
            .with_max_threads(0);
        
        let engine = builder.build();
        assert!(engine.is_err());
    }

    #[test]
    fn test_get_execution_plan() {
        let config = ProcessingConfig::default();
        let engine = ProcessingEngine::new(config);
        
        let plan = engine.get_execution_plan();
        assert!(plan.is_ok());
        
        let plan = plan.unwrap();
        assert!(!plan.is_empty());
    }

    #[test]
    fn test_get_status() {
        let config = ProcessingConfig::default();
        let engine = ProcessingEngine::new(config.clone());
        
        let status = engine.get_status();
        assert_eq!(status.config.strategy, config.strategy);
        assert_eq!(status.stats.total_runs, 0);
    }

    #[test]
    fn test_reset_stats() {
        let config = ProcessingConfig::default();
        let mut engine = ProcessingEngine::new(config);
        
        // Manually update stats
        engine.stats.total_runs = 5;
        engine.stats.successful_runs = 3;
        
        engine.reset_stats();
        
        assert_eq!(engine.stats().total_runs, 0);
        assert_eq!(engine.stats().successful_runs, 0);
    }

    #[test]
    fn test_engine_with_custom_pipeline() {
        let config = ProcessingConfig::default();
        let pipeline = Box::new(DefaultPipeline::new(config.clone()));
        let engine = ProcessingEngine::with_pipeline(config, pipeline);
        
        assert_eq!(engine.stats().total_runs, 0);
    }
}