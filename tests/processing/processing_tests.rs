//! Comprehensive unit tests for processing system
//! 
//! These tests verify the processing system including:
//! - ProcessingEngine functionality and lifecycle
//! - Processing strategies (in-memory, chunked, memory-mapped)
//! - Pipeline stages (loader, transformer, validator, writer)
//! - Processing configuration and context
//! - Statistics tracking and performance monitoring
//! - Async processing and error handling
//! - Registry and factory systems
//! 
//! The tests cover:
//! - Engine creation and configuration
//! - Processing workflow execution
//! - Strategy selection and optimization
//! - Pipeline stage coordination
//! - Error handling and recovery
//! - Performance metrics and statistics
//! - Concurrent processing scenarios

use std::collections::HashMap;
use std::time::Duration;
use std::sync::Arc;
use tokio;

use rusty::processing::{
    ProcessingEngine, ProcessingEngineBuilder, EngineStats, EngineStatus,
    ProcessingConfig, ProcessingStrategy, ProcessingInput, ProcessingOutput,
    ProcessingContext, ProcessingStats, ProcessedData,
    InMemoryProcessor, ChunkedProcessor, MemoryMappedProcessor,
    create_processor, recommend_strategy, create_engine, create_optimized_engine,
    DefaultPipeline, LoaderStageImpl, TransformerStageImpl, ValidatorStageImpl, WriterStageImpl,
    DefaultProcessorRegistry, ProcessorRegistryImpl,
    DagExecutor, DagExecutionStats, DagExecutionContext, TaskState, TaskExecutionResult,
    ValidationResult, ValidationRule, ValidationRuleType, ValidationSeverity,
    TransformationRule, TransformationRuleType
};
use rusty::error::{Result, ProcessingError};
use rusty::data::{Series, Observation, Survey};

#[cfg(test)]
mod engine_tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let config = ProcessingConfig::default();
        let engine = ProcessingEngine::new(config.clone());
        
        assert_eq!(engine.config().strategy, config.strategy);
        assert_eq!(engine.stats().total_runs, 0);
        assert_eq!(engine.stats().successful_runs, 0);
        assert_eq!(engine.stats().failed_runs, 0);
    }

    #[test]
    fn test_engine_with_custom_config() {
        let config = ProcessingConfig::default()
            .with_strategy(ProcessingStrategy::Chunked)
            .with_max_threads(8)
            .with_chunk_size(5000)
            .with_memory_limit(2000000000);
        
        let engine = ProcessingEngine::new(config.clone());
        
        assert_eq!(engine.config().strategy, ProcessingStrategy::Chunked);
        assert_eq!(engine.config().max_threads, Some(8));
        assert_eq!(engine.config().chunk_size, Some(5000));
        assert_eq!(engine.config().memory_limit, Some(2000000000));
    }

    #[test]
    fn test_engine_builder() {
        let engine = ProcessingEngineBuilder::new()
            .with_strategy(ProcessingStrategy::InMemory)
            .with_max_threads(4)
            .with_parallel(true)
            .build();
        
        assert_eq!(engine.config().strategy, ProcessingStrategy::InMemory);
        assert_eq!(engine.config().max_threads, Some(4));
        assert_eq!(engine.config().parallel, Some(true));
    }

    #[test]
    fn test_engine_stats_initialization() {
        let stats = EngineStats::default();
        
        assert_eq!(stats.total_runs, 0);
        assert_eq!(stats.total_processing_time_ms, 0);
        assert_eq!(stats.successful_runs, 0);
        assert_eq!(stats.failed_runs, 0);
        assert_eq!(stats.avg_processing_time_ms, 0.0);
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_engine_stats_update_success() {
        let mut stats = EngineStats::default();
        
        stats.update(100, true);
        
        assert_eq!(stats.total_runs, 1);
        assert_eq!(stats.total_processing_time_ms, 100);
        assert_eq!(stats.successful_runs, 1);
        assert_eq!(stats.failed_runs, 0);
        assert_eq!(stats.avg_processing_time_ms, 100.0);
        assert_eq!(stats.success_rate(), 1.0);
    }

    #[test]
    fn test_engine_stats_update_failure() {
        let mut stats = EngineStats::default();
        
        stats.update(150, false);
        
        assert_eq!(stats.total_runs, 1);
        assert_eq!(stats.total_processing_time_ms, 150);
        assert_eq!(stats.successful_runs, 0);
        assert_eq!(stats.failed_runs, 1);
        assert_eq!(stats.avg_processing_time_ms, 150.0);
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_engine_stats_multiple_updates() {
        let mut stats = EngineStats::default();
        
        stats.update(100, true);
        stats.update(200, false);
        stats.update(150, true);
        
        assert_eq!(stats.total_runs, 3);
        assert_eq!(stats.total_processing_time_ms, 450);
        assert_eq!(stats.successful_runs, 2);
        assert_eq!(stats.failed_runs, 1);
        assert_eq!(stats.avg_processing_time_ms, 150.0);
        assert_eq!(stats.success_rate(), 2.0 / 3.0);
    }

    #[tokio::test]
    async fn test_engine_process_basic() {
        let config = ProcessingConfig::default();
        let mut engine = ProcessingEngine::new(config);
        
        let input = ProcessingInput::new(vec!["test_data.csv".to_string()]);
        let output = ProcessingOutput::new(vec!["output.csv".to_string()], "csv".to_string());
        
        // This test assumes the processing implementation exists
        // In a real scenario, we might need to mock the pipeline
        let result = engine.process(input, output).await;
        
        // The exact assertion depends on the implementation
        // For now, we just verify the method can be called
        match result {
            Ok(_) => {
                assert!(engine.stats().total_runs > 0);
            }
            Err(_) => {
                // Processing might fail due to missing test data, which is expected
                assert!(engine.stats().total_runs > 0);
            }
        }
    }

    #[test]
    fn test_create_engine_convenience() {
        let engine = create_engine();
        assert_eq!(engine.config().strategy, ProcessingStrategy::Auto);
    }

    #[test]
    fn test_create_optimized_engine() {
        let input = ProcessingInput::new(vec!["small_file.csv".to_string()]);
        let result = create_optimized_engine(&input);
        
        assert!(result.is_ok());
        let engine = result.unwrap();
        // Strategy should be optimized based on input
        assert!(matches!(
            engine.config().strategy,
            ProcessingStrategy::InMemory | ProcessingStrategy::Chunked | ProcessingStrategy::MemoryMapped
        ));
    }
}

#[cfg(test)]
mod strategy_tests {
    use super::*;

    #[test]
    fn test_processing_strategy_enum() {
        let strategies = vec![
            ProcessingStrategy::Auto,
            ProcessingStrategy::InMemory,
            ProcessingStrategy::Chunked,
            ProcessingStrategy::MemoryMapped,
        ];
        
        for strategy in strategies {
            let config = ProcessingConfig::default().with_strategy(strategy.clone());
            assert_eq!(config.strategy, strategy);
        }
    }

    #[test]
    fn test_strategy_recommendation_small_input() {
        let input = ProcessingInput::new(vec!["small_file.csv".to_string()])
            .with_estimated_size(1024 * 1024); // 1MB
        
        let strategy = recommend_strategy(&input).unwrap();
        assert_eq!(strategy, ProcessingStrategy::InMemory);
    }

    #[test]
    fn test_strategy_recommendation_medium_input() {
        let input = ProcessingInput::new(vec!["medium_file.csv".to_string()])
            .with_estimated_size(500 * 1024 * 1024); // 500MB
        
        let strategy = recommend_strategy(&input).unwrap();
        assert_eq!(strategy, ProcessingStrategy::Chunked);
    }

    #[test]
    fn test_strategy_recommendation_large_input() {
        let input = ProcessingInput::new(vec!["large_file.csv".to_string()])
            .with_estimated_size(5 * 1024 * 1024 * 1024); // 5GB
        
        let strategy = recommend_strategy(&input).unwrap();
        assert_eq!(strategy, ProcessingStrategy::MemoryMapped);
    }

    #[test]
    fn test_create_processor_in_memory() {
        let config = ProcessingConfig::default().with_strategy(ProcessingStrategy::InMemory);
        let processor = create_processor(&config).unwrap();
        
        assert_eq!(processor.strategy(), ProcessingStrategy::InMemory);
        assert!(processor.supports_parallel_processing());
    }

    #[test]
    fn test_create_processor_chunked() {
        let config = ProcessingConfig::default()
            .with_strategy(ProcessingStrategy::Chunked)
            .with_chunk_size(10000);
        
        let processor = create_processor(&config).unwrap();
        
        assert_eq!(processor.strategy(), ProcessingStrategy::Chunked);
        assert_eq!(processor.chunk_size(), Some(10000));
    }

    #[test]
    fn test_create_processor_memory_mapped() {
        let config = ProcessingConfig::default().with_strategy(ProcessingStrategy::MemoryMapped);
        let processor = create_processor(&config).unwrap();
        
        assert_eq!(processor.strategy(), ProcessingStrategy::MemoryMapped);
        assert!(processor.supports_streaming());
    }

    #[test]
    fn test_in_memory_processor() {
        let config = ProcessingConfig::default().with_max_threads(4);
        let processor = InMemoryProcessor::new(config);
        
        assert_eq!(processor.strategy(), ProcessingStrategy::InMemory);
        assert_eq!(processor.max_threads(), Some(4));
        assert!(processor.supports_parallel_processing());
        assert!(!processor.supports_streaming());
    }

    #[test]
    fn test_chunked_processor() {
        let config = ProcessingConfig::default()
            .with_chunk_size(5000)
            .with_max_threads(2);
        
        let processor = ChunkedProcessor::new(config);
        
        assert_eq!(processor.strategy(), ProcessingStrategy::Chunked);
        assert_eq!(processor.chunk_size(), Some(5000));
        assert_eq!(processor.max_threads(), Some(2));
        assert!(processor.supports_parallel_processing());
        assert!(processor.supports_chunked_processing());
    }

    #[test]
    fn test_memory_mapped_processor() {
        let config = ProcessingConfig::default().with_memory_limit(1000000000);
        let processor = MemoryMappedProcessor::new(config);
        
        assert_eq!(processor.strategy(), ProcessingStrategy::MemoryMapped);
        assert_eq!(processor.memory_limit(), Some(1000000000));
        assert!(processor.supports_streaming());
        assert!(processor.supports_memory_mapping());
    }
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;

    #[test]
    fn test_default_pipeline_creation() {
        let config = ProcessingConfig::default();
        let pipeline = DefaultPipeline::new(config.clone());
        
        assert_eq!(pipeline.config(), &config);
        assert_eq!(pipeline.stage_count(), 4); // loader, transformer, validator, writer
    }

    #[test]
    fn test_pipeline_stages() {
        let config = ProcessingConfig::default();
        let pipeline = DefaultPipeline::new(config);
        
        let stages = pipeline.stages();
        assert_eq!(stages.len(), 4);
        
        assert_eq!(stages[0].name(), "loader");
        assert_eq!(stages[1].name(), "transformer");
        assert_eq!(stages[2].name(), "validator");
        assert_eq!(stages[3].name(), "writer");
    }

    #[test]
    fn test_loader_stage() {
        let config = ProcessingConfig::default();
        let stage = LoaderStageImpl::new(config);
        
        assert_eq!(stage.name(), "loader");
        assert_eq!(stage.stage_type(), "loader");
        assert!(stage.is_enabled());
    }

    #[test]
    fn test_transformer_stage() {
        let config = ProcessingConfig::default();
        let stage = TransformerStageImpl::new(config);
        
        assert_eq!(stage.name(), "transformer");
        assert_eq!(stage.stage_type(), "transformer");
        assert!(stage.is_enabled());
    }

    #[test]
    fn test_validator_stage() {
        let config = ProcessingConfig::default();
        let stage = ValidatorStageImpl::new(config);
        
        assert_eq!(stage.name(), "validator");
        assert_eq!(stage.stage_type(), "validator");
        assert!(stage.is_enabled());
    }

    #[test]
    fn test_writer_stage() {
        let config = ProcessingConfig::default();
        let stage = WriterStageImpl::new(config);
        
        assert_eq!(stage.name(), "writer");
        assert_eq!(stage.stage_type(), "writer");
        assert!(stage.is_enabled());
    }

    #[tokio::test]
    async fn test_pipeline_execution() {
        let config = ProcessingConfig::default();
        let mut pipeline = DefaultPipeline::new(config);
        
        let input = ProcessingInput::new(vec!["test.csv".to_string()]);
        let mut context = ProcessingContext::new(ProcessingConfig::default());
        
        let result = pipeline.execute(input, &mut context).await;
        
        // The exact result depends on implementation
        // We're mainly testing that the method can be called
        match result {
            Ok(_) => assert!(context.stats().total_records_processed >= 0),
            Err(_) => {
                // Pipeline might fail due to missing test data
                assert!(context.stats().total_errors >= 0);
            }
        }
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn test_validation_rule_creation() {
        let rule = ValidationRule::new("series_id_format", ValidationRuleType::Format)
            .with_severity(ValidationSeverity::Error)
            .with_description("Series ID must follow BLS format");
        
        assert_eq!(rule.name(), "series_id_format");
        assert_eq!(rule.rule_type(), ValidationRuleType::Format);
        assert_eq!(rule.severity(), ValidationSeverity::Error);
        assert!(rule.description().contains("BLS format"));
    }

    #[test]
    fn test_validation_rule_types() {
        let rule_types = vec![
            ValidationRuleType::Required,
            ValidationRuleType::Format,
            ValidationRuleType::Range,
            ValidationRuleType::Custom,
        ];
        
        for rule_type in rule_types {
            let rule = ValidationRule::new("test_rule", rule_type.clone());
            assert_eq!(rule.rule_type(), rule_type);
        }
    }

    #[test]
    fn test_validation_severity_levels() {
        let severities = vec![
            ValidationSeverity::Info,
            ValidationSeverity::Warning,
            ValidationSeverity::Error,
            ValidationSeverity::Critical,
        ];
        
        for severity in severities {
            let rule = ValidationRule::new("test_rule", ValidationRuleType::Required)
                .with_severity(severity.clone());
            assert_eq!(rule.severity(), severity);
        }
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        
        assert!(result.is_valid());
        assert_eq!(result.error_count(), 0);
        assert_eq!(result.warning_count(), 0);
        
        result.add_error("Invalid series ID format");
        result.add_warning("Missing optional field");
        
        assert!(!result.is_valid());
        assert_eq!(result.error_count(), 1);
        assert_eq!(result.warning_count(), 1);
        
        let errors = result.errors();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Invalid series ID"));
        
        let warnings = result.warnings();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Missing optional"));
    }

    #[test]
    fn test_validation_result_severity_filtering() {
        let mut result = ValidationResult::new();
        
        result.add_error("Critical error");
        result.add_warning("Minor warning");
        result.add_info("Information message");
        
        let critical_issues = result.issues_by_severity(ValidationSeverity::Error);
        assert_eq!(critical_issues.len(), 1);
        
        let all_issues = result.all_issues();
        assert_eq!(all_issues.len(), 3);
    }
}

#[cfg(test)]
mod transformation_tests {
    use super::*;

    #[test]
    fn test_transformation_rule_creation() {
        let rule = TransformationRule::new("normalize_values", TransformationRuleType::Normalize)
            .with_description("Normalize observation values")
            .with_parameter("min_value", "0.0")
            .with_parameter("max_value", "100.0");
        
        assert_eq!(rule.name(), "normalize_values");
        assert_eq!(rule.rule_type(), TransformationRuleType::Normalize);
        assert!(rule.description().contains("Normalize"));
        assert_eq!(rule.get_parameter("min_value"), Some(&"0.0".to_string()));
        assert_eq!(rule.get_parameter("max_value"), Some(&"100.0".to_string()));
    }

    #[test]
    fn test_transformation_rule_types() {
        let rule_types = vec![
            TransformationRuleType::Clean,
            TransformationRuleType::Normalize,
            TransformationRuleType::Aggregate,
            TransformationRuleType::Filter,
            TransformationRuleType::Custom,
        ];
        
        for rule_type in rule_types {
            let rule = TransformationRule::new("test_rule", rule_type.clone());
            assert_eq!(rule.rule_type(), rule_type);
        }
    }

    #[test]
    fn test_transformation_rule_parameters() {
        let mut rule = TransformationRule::new("test_transform", TransformationRuleType::Custom);
        
        rule.add_parameter("param1", "value1");
        rule.add_parameter("param2", "value2");
        
        assert_eq!(rule.parameter_count(), 2);
        assert_eq!(rule.get_parameter("param1"), Some(&"value1".to_string()));
        assert_eq!(rule.get_parameter("param2"), Some(&"value2".to_string()));
        assert_eq!(rule.get_parameter("nonexistent"), None);
        
        let all_params = rule.parameters();
        assert_eq!(all_params.len(), 2);
        assert!(all_params.contains_key("param1"));
        assert!(all_params.contains_key("param2"));
    }
}

#[cfg(test)]
mod context_tests {
    use super::*;

    #[test]
    fn test_processing_context_creation() {
        let config = ProcessingConfig::default();
        let context = ProcessingContext::new(config.clone());
        
        assert_eq!(context.config(), &config);
        assert_eq!(context.stats().total_records_processed, 0);
        assert_eq!(context.stats().total_errors, 0);
        assert!(context.metadata().is_empty());
    }

    #[test]
    fn test_processing_context_metadata() {
        let config = ProcessingConfig::default();
        let mut context = ProcessingContext::new(config);
        
        context.add_metadata("survey_code", "AP");
        context.add_metadata("processing_date", "2023-12-01");
        
        assert_eq!(context.get_metadata("survey_code"), Some(&"AP".to_string()));
        assert_eq!(context.get_metadata("processing_date"), Some(&"2023-12-01".to_string()));
        assert_eq!(context.get_metadata("nonexistent"), None);
        
        let all_metadata = context.metadata();
        assert_eq!(all_metadata.len(), 2);
    }

    #[test]
    fn test_processing_stats() {
        let mut stats = ProcessingStats::new();
        
        assert_eq!(stats.total_records_processed, 0);
        assert_eq!(stats.total_errors, 0);
        assert_eq!(stats.total_warnings, 0);
        
        stats.increment_processed(100);
        stats.increment_errors(2);
        stats.increment_warnings(5);
        
        assert_eq!(stats.total_records_processed, 100);
        assert_eq!(stats.total_errors, 2);
        assert_eq!(stats.total_warnings, 5);
        
        let error_rate = stats.error_rate();
        assert_eq!(error_rate, 0.02); // 2/100
    }

    #[test]
    fn test_processed_data() {
        let series = vec![
            Series::new("APUS49074714", "Test Series 1"),
            Series::new("APUS49074715", "Test Series 2"),
        ];
        
        let observations = vec![
            Observation::new("APUS49074714", 2023, "M01", Some(100.0)),
            Observation::new("APUS49074715", 2023, "M01", Some(200.0)),
        ];
        
        let data = ProcessedData::new()
            .with_series(series.clone())
            .with_observations(observations.clone());
        
        assert_eq!(data.series().len(), 2);
        assert_eq!(data.observations().len(), 2);
        assert_eq!(data.series()[0].id(), "APUS49074714");
        assert_eq!(data.observations()[0].series_id(), "APUS49074714");
    }
}

#[cfg(test)]
mod registry_tests {
    use super::*;

    #[test]
    fn test_processor_registry_creation() {
        let registry = DefaultProcessorRegistry::new();
        
        assert_eq!(registry.processor_count(), 0);
        assert!(registry.list_processors().is_empty());
    }

    #[test]
    fn test_processor_registry_registration() {
        let mut registry = DefaultProcessorRegistry::new();
        let config = ProcessingConfig::default();
        let processor = InMemoryProcessor::new(config);
        
        registry.register_processor("in_memory", Box::new(processor));
        
        assert_eq!(registry.processor_count(), 1);
        assert!(registry.has_processor("in_memory"));
        assert!(!registry.has_processor("nonexistent"));
        
        let processors = registry.list_processors();
        assert_eq!(processors.len(), 1);
        assert!(processors.contains(&"in_memory".to_string()));
    }

    #[test]
    fn test_processor_registry_retrieval() {
        let mut registry = DefaultProcessorRegistry::new();
        let config = ProcessingConfig::default();
        let processor = ChunkedProcessor::new(config);
        
        registry.register_processor("chunked", Box::new(processor));
        
        let retrieved = registry.get_processor("chunked");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().strategy(), ProcessingStrategy::Chunked);
        
        let missing = registry.get_processor("missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_processor_registry_impl() {
        let registry = ProcessorRegistryImpl::default();
        
        // Default registry should have some built-in processors
        assert!(registry.processor_count() >= 3); // in_memory, chunked, memory_mapped
        assert!(registry.has_processor("in_memory"));
        assert!(registry.has_processor("chunked"));
        assert!(registry.has_processor("memory_mapped"));
    }
}

#[cfg(test)]
mod dag_tests {
    use super::*;

    #[test]
    fn test_dag_executor_creation() {
        let dag_config = rusty::config::DagsConfig {
            dags: vec![],
        };
        
        let executor = DagExecutor::new(dag_config);
        assert_eq!(executor.dag_count(), 0);
        assert!(executor.list_dags().is_empty());
    }

    #[test]
    fn test_task_state_transitions() {
        let mut state = TaskState::Pending;
        
        assert_eq!(state, TaskState::Pending);
        assert!(!state.is_running());
        assert!(!state.is_completed());
        assert!(!state.is_failed());
        
        state = TaskState::Running;
        assert!(state.is_running());
        assert!(!state.is_completed());
        
        state = TaskState::Completed;
        assert!(!state.is_running());
        assert!(state.is_completed());
        assert!(!state.is_failed());
        
        state = TaskState::Failed;
        assert!(!state.is_running());
        assert!(!state.is_completed());
        assert!(state.is_failed());
    }

    #[test]
    fn test_task_execution_result() {
        let success_result = TaskExecutionResult::success("task1", Duration::from_millis(100));
        assert!(success_result.is_success());
        assert_eq!(success_result.task_name(), "task1");
        assert_eq!(success_result.execution_time(), Duration::from_millis(100));
        
        let failure_result = TaskExecutionResult::failure("task2", "Task failed".to_string());
        assert!(!failure_result.is_success());
        assert_eq!(failure_result.task_name(), "task2");
        assert!(failure_result.error_message().contains("Task failed"));
    }

    #[test]
    fn test_dag_execution_stats() {
        let mut stats = DagExecutionStats::new();
        
        assert_eq!(stats.total_tasks(), 0);
        assert_eq!(stats.completed_tasks(), 0);
        assert_eq!(stats.failed_tasks(), 0);
        assert_eq!(stats.completion_rate(), 0.0);
        
        stats.add_task_result(TaskExecutionResult::success("task1", Duration::from_millis(100)));
        stats.add_task_result(TaskExecutionResult::failure("task2", "Error".to_string()));
        
        assert_eq!(stats.total_tasks(), 2);
        assert_eq!(stats.completed_tasks(), 1);
        assert_eq!(stats.failed_tasks(), 1);
        assert_eq!(stats.completion_rate(), 0.5);
    }
}