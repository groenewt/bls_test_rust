//! # Default Pipeline Implementation
//!
//! This module provides the default pipeline implementation that orchestrates
//! all processing stages in a coordinated manner.

use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

use crate::error::types::{Error, ProcessingError, Result};
use crate::processing::traits::{
    PipelineStage, ProcessingConfig, ProcessingContext, ProcessingPipeline,
};

/// Default implementation of the processing pipeline
pub struct DefaultPipeline {
    /// Configuration for the pipeline
    config: ProcessingConfig,
    /// Ordered list of pipeline stages
    stages: Vec<Box<dyn PipelineStage>>,
    /// Pipeline execution statistics
    stats: PipelineStats,
}

/// Statistics for pipeline execution
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Number of stages executed
    pub stages_executed: u32,
    /// Number of errors encountered
    pub error_count: u32,
    /// Stage execution times
    pub stage_times: HashMap<String, u64>,
}

impl DefaultPipeline {
    /// Create a new default pipeline with the given configuration
    pub fn new(config: ProcessingConfig) -> Self {
        Self {
            config,
            stages: Vec::new(),
            stats: PipelineStats::default(),
        }
    }

    /// Get pipeline statistics
    pub fn stats(&self) -> &PipelineStats {
        &self.stats
    }

    /// Reset pipeline statistics
    pub fn reset_stats(&mut self) {
        self.stats = PipelineStats::default();
    }

    /// Check stage dependencies
    fn check_dependencies(&self) -> Result<()> {
        let stage_names: Vec<String> = self.stages.iter().map(|s| s.name().to_string()).collect();

        for stage in &self.stages {
            for dependency in stage.dependencies() {
                if !stage_names.contains(&dependency) {
                    return Err(Error::Processing(ProcessingError::InvalidConfiguration(
                        format!(
                            "Stage '{}' depends on '{}' which is not present in the pipeline",
                            stage.name(),
                            dependency
                        ),
                    )));
                }
            }
        }

        Ok(())
    }

    /// Sort stages by dependencies
    fn sort_stages_by_dependencies(&mut self) -> Result<()> {
        // Simple topological sort implementation
        let mut sorted_stages = Vec::new();
        let mut remaining_stages = std::mem::take(&mut self.stages);

        while !remaining_stages.is_empty() {
            let mut found_stage = false;

            for i in 0..remaining_stages.len() {
                let stage = &remaining_stages[i];
                let dependencies = stage.dependencies();

                // Check if all dependencies are already in sorted_stages
                let all_deps_satisfied = dependencies.iter().all(|dep| {
                    sorted_stages
                        .iter()
                        .any(|s: &Box<dyn PipelineStage>| s.name() == dep)
                });

                if all_deps_satisfied {
                    let stage = remaining_stages.remove(i);
                    sorted_stages.push(stage);
                    found_stage = true;
                    break;
                }
            }

            if !found_stage {
                return Err(Error::Processing(ProcessingError::InvalidConfiguration(
                    "Circular dependency detected in pipeline stages".to_string(),
                )));
            }
        }

        self.stages = sorted_stages;
        Ok(())
    }
}

#[async_trait]
impl ProcessingPipeline for DefaultPipeline {
    fn add_stage(&mut self, stage: Box<dyn PipelineStage>) -> Result<()> {
        // Check for duplicate stage names
        if self.stages.iter().any(|s| s.name() == stage.name()) {
            return Err(Error::Processing(ProcessingError::InvalidConfiguration(
                format!("Stage '{}' already exists in the pipeline", stage.name()),
            )));
        }

        self.stages.push(stage);

        // Re-sort stages by dependencies
        self.sort_stages_by_dependencies()?;

        Ok(())
    }

    fn remove_stage(&mut self, stage_name: &str) -> Result<()> {
        let initial_len = self.stages.len();
        self.stages.retain(|stage| stage.name() != stage_name);

        if self.stages.len() == initial_len {
            return Err(Error::Processing(ProcessingError::InvalidConfiguration(
                format!("Stage '{stage_name}' not found in the pipeline"),
            )));
        }

        Ok(())
    }

    fn get_stages(&self) -> Vec<&dyn PipelineStage> {
        self.stages.iter().map(|s| s.as_ref()).collect()
    }

    async fn execute(&mut self, context: &mut ProcessingContext) -> Result<()> {
        let start_time = Instant::now();

        log::info!(
            "Starting pipeline execution with {} stages",
            self.stages.len()
        );

        // Check dependencies before execution
        self.check_dependencies()?;

        // Validate all stages before execution
        for stage in &self.stages {
            if let Err(e) = stage.validate(context) {
                log::error!("Stage '{}' validation failed: {}", stage.name(), e);
                return Err(e);
            }
        }

        // Execute stages in order
        for stage in &mut self.stages {
            let stage_start_time = Instant::now();

            log::info!("Executing stage: {}", stage.name());

            // Check if stage can process the current context
            if !stage.can_process(context)? {
                log::warn!(
                    "Stage '{}' cannot process current context, skipping",
                    stage.name()
                );
                continue;
            }

            // Execute the stage
            match stage.execute(context).await {
                Ok(()) => {
                    let stage_elapsed = stage_start_time.elapsed();
                    self.stats
                        .stage_times
                        .insert(stage.name().to_string(), stage_elapsed.as_millis() as u64);
                    self.stats.stages_executed += 1;

                    log::info!(
                        "Stage '{}' completed in {}ms",
                        stage.name(),
                        stage_elapsed.as_millis()
                    );
                }
                Err(e) => {
                    self.stats.error_count += 1;
                    log::error!("Stage '{}' failed: {}", stage.name(), e);

                    // Attempt cleanup for the failed stage
                    if let Err(cleanup_err) = stage.cleanup(context).await {
                        log::error!(
                            "Cleanup failed for stage '{}': {}",
                            stage.name(),
                            cleanup_err
                        );
                    }

                    return Err(e);
                }
            }
        }

        // Cleanup all stages
        for stage in &mut self.stages {
            if let Err(e) = stage.cleanup(context).await {
                log::warn!("Cleanup warning for stage '{}': {}", stage.name(), e);
            }
        }

        let total_elapsed = start_time.elapsed();
        self.stats.total_execution_time_ms = total_elapsed.as_millis() as u64;

        log::info!(
            "Pipeline execution completed in {}ms",
            total_elapsed.as_millis()
        );

        Ok(())
    }

    fn validate(&self, context: &ProcessingContext) -> Result<()> {
        // Check that we have at least one stage
        if self.stages.is_empty() {
            return Err(Error::Processing(ProcessingError::InvalidConfiguration(
                "Pipeline has no stages".to_string(),
            )));
        }

        // Check dependencies
        self.check_dependencies()?;

        // Validate each stage
        for stage in &self.stages {
            stage.validate(context)?;
        }

        Ok(())
    }

    fn get_execution_plan(&self) -> Result<Vec<String>> {
        let mut plan = Vec::new();

        for stage in &self.stages {
            plan.push(format!("{}: {}", stage.name(), stage.description()));
        }

        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::pipeline::{
        LoaderStageImpl, TransformerStageImpl, ValidatorStageImpl, WriterStageImpl,
    };

    #[test]
    fn test_default_pipeline_creation() {
        let config = ProcessingConfig::default();
        let pipeline = DefaultPipeline::new(config);

        assert_eq!(pipeline.stages.len(), 0);
        assert_eq!(pipeline.stats.stages_executed, 0);
    }

    #[test]
    fn test_add_stage() {
        let config = ProcessingConfig::default();
        let mut pipeline = DefaultPipeline::new(config);

        let loader = Box::new(LoaderStageImpl::new());
        assert!(pipeline.add_stage(loader).is_ok());
        assert_eq!(pipeline.stages.len(), 1);
    }

    #[test]
    fn test_add_duplicate_stage() {
        let config = ProcessingConfig::default();
        let mut pipeline = DefaultPipeline::new(config);

        let loader1 = Box::new(LoaderStageImpl::new());
        let loader2 = Box::new(LoaderStageImpl::new());

        assert!(pipeline.add_stage(loader1).is_ok());
        assert!(pipeline.add_stage(loader2).is_err());
    }

    #[test]
    fn test_remove_stage() {
        let config = ProcessingConfig::default();
        let mut pipeline = DefaultPipeline::new(config);

        let loader = Box::new(LoaderStageImpl::new());
        pipeline.add_stage(loader).unwrap();

        assert!(pipeline.remove_stage("loader").is_ok());
        assert_eq!(pipeline.stages.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_stage() {
        let config = ProcessingConfig::default();
        let mut pipeline = DefaultPipeline::new(config);

        assert!(pipeline.remove_stage("nonexistent").is_err());
    }

    #[test]
    fn test_get_stages() {
        let config = ProcessingConfig::default();
        let mut pipeline = DefaultPipeline::new(config);

        let loader = Box::new(LoaderStageImpl::new());
        pipeline.add_stage(loader).unwrap();

        let stages = pipeline.get_stages();
        assert_eq!(stages.len(), 1);
        assert_eq!(stages[0].name(), "loader");
    }

    #[test]
    fn test_get_execution_plan() {
        let config = ProcessingConfig::default();
        let mut pipeline = DefaultPipeline::new(config);

        let loader = Box::new(LoaderStageImpl::new());
        pipeline.add_stage(loader).unwrap();

        let plan = pipeline.get_execution_plan().unwrap();
        assert_eq!(plan.len(), 1);
        assert!(plan[0].contains("loader"));
    }

    #[test]
    fn test_validate_empty_pipeline() {
        let config = ProcessingConfig::default();
        let pipeline = DefaultPipeline::new(config.clone());
        let context = ProcessingContext::new(config);

        assert!(pipeline.validate(&context).is_err());
    }

    #[test]
    fn test_stage_dependency_sorting() {
        let config = ProcessingConfig::default();
        let mut pipeline = DefaultPipeline::new(config);

        // Add stages in reverse dependency order
        let writer = Box::new(WriterStageImpl::new());
        let validator = Box::new(ValidatorStageImpl::new());
        let transformer = Box::new(TransformerStageImpl::new());
        let loader = Box::new(LoaderStageImpl::new());

        pipeline.add_stage(writer).unwrap();
        pipeline.add_stage(validator).unwrap();
        pipeline.add_stage(transformer).unwrap();
        pipeline.add_stage(loader).unwrap();

        // Stages should be sorted by dependencies
        let stages = pipeline.get_stages();
        assert_eq!(stages[0].name(), "loader");
        assert_eq!(stages[1].name(), "transformer");
        assert_eq!(stages[2].name(), "validator");
        assert_eq!(stages[3].name(), "writer");
    }

    #[test]
    fn test_reset_stats() {
        let config = ProcessingConfig::default();
        let mut pipeline = DefaultPipeline::new(config);

        pipeline.stats.stages_executed = 5;
        pipeline.stats.error_count = 2;

        pipeline.reset_stats();

        assert_eq!(pipeline.stats.stages_executed, 0);
        assert_eq!(pipeline.stats.error_count, 0);
    }
}
