//! Comprehensive unit tests for error handling system
//! 
//! These tests verify the error handling system including:
//! - Error type hierarchy and variants
//! - Error creation and conversion
//! - Error context and location tracking
//! - Error serialization and deserialization
//! - Error recovery mechanisms
//! - Error telemetry and monitoring
//! - Error sanitization and security
//! 
//! The tests cover:
//! - All error type variants and their fields
//! - Error conversion between types
//! - Error context functionality
//! - Error display and formatting
//! - Error chaining and source tracking
//! - Enterprise error handling features

use std::collections::HashMap;
use std::error::Error as StdError;
use serde_json;

use rusty::error::{
    Error, Result,
    ConfigError, DataError, ProcessingError, OutputError, PluginError, SystemError,
    ErrorContext, ContextExt, OptionContextExt,
    RetryPolicy, CircuitBreaker, RecoveryStrategy,
    ErrorMetrics, ErrorCollector,
    ErrorSanitizer, SanitizationPolicy,
    ErrorLocalizer, MessageCatalog
};

#[cfg(test)]
mod error_types_tests {
    use super::*;

    #[test]
    fn test_config_error_creation() {
        let load_error = ConfigError::LoadError {
            path: "/path/to/config.yml".to_string(),
            source: "File not found".to_string(),
        };
        
        assert!(matches!(load_error, ConfigError::LoadError { .. }));
        if let ConfigError::LoadError { path, source } = load_error {
            assert_eq!(path, "/path/to/config.yml");
            assert_eq!(source, "File not found");
        }
    }

    #[test]
    fn test_config_error_validation() {
        let validation_error = ConfigError::ValidationError {
            message: "Invalid survey code".to_string(),
            field: Some("survey_code".to_string()),
        };
        
        assert!(matches!(validation_error, ConfigError::ValidationError { .. }));
        if let ConfigError::ValidationError { message, field } = validation_error {
            assert_eq!(message, "Invalid survey code");
            assert_eq!(field, Some("survey_code".to_string()));
        }
    }

    #[test]
    fn test_config_error_parse() {
        let parse_error = ConfigError::ParseError {
            message: "Invalid YAML syntax".to_string(),
            line: Some(42),
            column: Some(15),
        };
        
        if let ConfigError::ParseError { message, line, column } = parse_error {
            assert_eq!(message, "Invalid YAML syntax");
            assert_eq!(line, Some(42));
            assert_eq!(column, Some(15));
        }
    }

    #[test]
    fn test_config_error_missing() {
        let missing_error = ConfigError::MissingError {
            key: "database_url".to_string(),
            section: Some("runtime".to_string()),
        };
        
        if let ConfigError::MissingError { key, section } = missing_error {
            assert_eq!(key, "database_url");
            assert_eq!(section, Some("runtime".to_string()));
        }
    }

    #[test]
    fn test_config_error_version_mismatch() {
        let version_error = ConfigError::ConfigVersionMismatch {
            file_path: "/config/survey.yml".to_string(),
            expected_version: 2,
            actual_version: 1,
        };
        
        if let ConfigError::ConfigVersionMismatch { file_path, expected_version, actual_version } = version_error {
            assert_eq!(file_path, "/config/survey.yml");
            assert_eq!(expected_version, 2);
            assert_eq!(actual_version, 1);
        }
    }

    #[test]
    fn test_config_error_dag_validation() {
        let dag_error = ConfigError::DagValidationError {
            dag_name: "processing_dag".to_string(),
            message: "Cycle detected in task dependencies".to_string(),
        };
        
        if let ConfigError::DagValidationError { dag_name, message } = dag_error {
            assert_eq!(dag_name, "processing_dag");
            assert_eq!(message, "Cycle detected in task dependencies");
        }
    }

    #[test]
    fn test_data_error_creation() {
        let read_error = DataError::ReadError {
            path: "/data/series.csv".to_string(),
            source: "Permission denied".to_string(),
        };
        
        if let DataError::ReadError { path, source } = read_error {
            assert_eq!(path, "/data/series.csv");
            assert_eq!(source, "Permission denied");
        }
    }

    #[test]
    fn test_data_error_validation() {
        let validation_error = DataError::ValidationError {
            message: "Invalid series ID format".to_string(),
            path: Some("/data/series.csv".to_string()),
            line: Some(123),
        };
        
        if let DataError::ValidationError { message, path, line } = validation_error {
            assert_eq!(message, "Invalid series ID format");
            assert_eq!(path, Some("/data/series.csv".to_string()));
            assert_eq!(line, Some(123));
        }
    }

    #[test]
    fn test_data_error_format() {
        let format_error = DataError::FormatError {
            message: "Expected CSV format".to_string(),
            expected: "CSV".to_string(),
            actual: "JSON".to_string(),
        };
        
        if let DataError::FormatError { message, expected, actual } = format_error {
            assert_eq!(message, "Expected CSV format");
            assert_eq!(expected, "CSV");
            assert_eq!(actual, "JSON");
        }
    }

    #[test]
    fn test_data_error_schema() {
        let schema_error = DataError::SchemaError {
            message: "Type mismatch".to_string(),
            field: "value".to_string(),
            expected_type: "f64".to_string(),
            actual_type: "String".to_string(),
        };
        
        if let DataError::SchemaError { message, field, expected_type, actual_type } = schema_error {
            assert_eq!(message, "Type mismatch");
            assert_eq!(field, "value");
            assert_eq!(expected_type, "f64");
            assert_eq!(actual_type, "String");
        }
    }

    #[test]
    fn test_processing_error_creation() {
        let transformation_error = ProcessingError::TransformationError {
            message: "Failed to transform data".to_string(),
            operation: "normalize_values".to_string(),
        };
        
        if let ProcessingError::TransformationError { message, operation } = transformation_error {
            assert_eq!(message, "Failed to transform data");
            assert_eq!(operation, "normalize_values");
        }
    }

    #[test]
    fn test_processing_error_computation() {
        let computation_error = ProcessingError::ComputationError {
            message: "Division by zero".to_string(),
            context: "calculating averages".to_string(),
        };
        
        if let ProcessingError::ComputationError { message, context } = computation_error {
            assert_eq!(message, "Division by zero");
            assert_eq!(context, "calculating averages");
        }
    }

    #[test]
    fn test_processing_error_strategy() {
        let strategy_error = ProcessingError::StrategyError {
            strategy: "in_memory".to_string(),
            message: "Insufficient memory".to_string(),
        };
        
        if let ProcessingError::StrategyError { strategy, message } = strategy_error {
            assert_eq!(strategy, "in_memory");
            assert_eq!(message, "Insufficient memory");
        }
    }

    #[test]
    fn test_processing_error_pipeline() {
        let pipeline_error = ProcessingError::PipelineError {
            stage: "validator".to_string(),
            message: "Validation failed".to_string(),
        };
        
        if let ProcessingError::PipelineError { stage, message } = pipeline_error {
            assert_eq!(stage, "validator");
            assert_eq!(message, "Validation failed");
        }
    }

    #[test]
    fn test_output_error_creation() {
        let write_error = OutputError::WriteError {
            path: "/output/result.csv".to_string(),
            source: "Disk full".to_string(),
        };
        
        if let OutputError::WriteError { path, source } = write_error {
            assert_eq!(path, "/output/result.csv");
            assert_eq!(source, "Disk full");
        }
    }

    #[test]
    fn test_output_error_format() {
        let format_error = OutputError::FormatError {
            format: "parquet".to_string(),
            message: "Invalid schema".to_string(),
        };
        
        if let OutputError::FormatError { format, message } = format_error {
            assert_eq!(format, "parquet");
            assert_eq!(message, "Invalid schema");
        }
    }

    #[test]
    fn test_output_error_serialization() {
        let serialization_error = OutputError::SerializationError {
            format: "json".to_string(),
            message: "Cannot serialize NaN values".to_string(),
        };
        
        if let OutputError::SerializationError { format, message } = serialization_error {
            assert_eq!(format, "json");
            assert_eq!(message, "Cannot serialize NaN values");
        }
    }

    #[test]
    fn test_output_error_partition() {
        let partition_error = OutputError::PartitionError {
            strategy: "time_based".to_string(),
            message: "Invalid partition key".to_string(),
        };
        
        if let OutputError::PartitionError { strategy, message } = partition_error {
            assert_eq!(strategy, "time_based");
            assert_eq!(message, "Invalid partition key");
        }
    }

    #[test]
    fn test_plugin_error_creation() {
        let load_error = PluginError::LoadError {
            plugin: "survey_processor".to_string(),
            source: "Symbol not found".to_string(),
        };
        
        if let PluginError::LoadError { plugin, source } = load_error {
            assert_eq!(plugin, "survey_processor");
            assert_eq!(source, "Symbol not found");
        }
    }

    #[test]
    fn test_plugin_error_execution() {
        let execution_error = PluginError::ExecutionError {
            plugin: "data_validator".to_string(),
            message: "Plugin crashed".to_string(),
        };
        
        if let PluginError::ExecutionError { plugin, message } = execution_error {
            assert_eq!(plugin, "data_validator");
            assert_eq!(message, "Plugin crashed");
        }
    }

    #[test]
    fn test_plugin_error_communication() {
        let communication_error = PluginError::CommunicationError {
            plugin: "remote_processor".to_string(),
            message: "Connection timeout".to_string(),
        };
        
        if let PluginError::CommunicationError { plugin, message } = communication_error {
            assert_eq!(plugin, "remote_processor");
            assert_eq!(message, "Connection timeout");
        }
    }

    #[test]
    fn test_plugin_error_configuration() {
        let config_error = PluginError::ConfigurationError {
            plugin: "output_formatter".to_string(),
            message: "Missing required parameter".to_string(),
        };
        
        if let PluginError::ConfigurationError { plugin, message } = config_error {
            assert_eq!(plugin, "output_formatter");
            assert_eq!(message, "Missing required parameter");
        }
    }

    #[test]
    fn test_system_error_creation() {
        let io_error = SystemError::IoError {
            operation: "read_file".to_string(),
            source: "No such file or directory".to_string(),
        };
        
        if let SystemError::IoError { operation, source } = io_error {
            assert_eq!(operation, "read_file");
            assert_eq!(source, "No such file or directory");
        }
    }
}

#[cfg(test)]
mod error_conversion_tests {
    use super::*;

    #[test]
    fn test_config_error_to_main_error() {
        let config_error = ConfigError::LoadError {
            path: "/config.yml".to_string(),
            source: "File not found".to_string(),
        };
        
        let main_error: Error = config_error.into();
        assert!(matches!(main_error, Error::Config(_)));
    }

    #[test]
    fn test_data_error_to_main_error() {
        let data_error = DataError::ReadError {
            path: "/data.csv".to_string(),
            source: "Permission denied".to_string(),
        };
        
        let main_error: Error = data_error.into();
        assert!(matches!(main_error, Error::Data(_)));
    }

    #[test]
    fn test_processing_error_to_main_error() {
        let processing_error = ProcessingError::TransformationError {
            message: "Transform failed".to_string(),
            operation: "normalize".to_string(),
        };
        
        let main_error: Error = processing_error.into();
        assert!(matches!(main_error, Error::Processing(_)));
    }

    #[test]
    fn test_output_error_to_main_error() {
        let output_error = OutputError::WriteError {
            path: "/output.csv".to_string(),
            source: "Disk full".to_string(),
        };
        
        let main_error: Error = output_error.into();
        assert!(matches!(main_error, Error::Output(_)));
    }

    #[test]
    fn test_plugin_error_to_main_error() {
        let plugin_error = PluginError::LoadError {
            plugin: "test_plugin".to_string(),
            source: "Library not found".to_string(),
        };
        
        let main_error: Error = plugin_error.into();
        assert!(matches!(main_error, Error::Plugin(_)));
    }

    #[test]
    fn test_system_error_to_main_error() {
        let system_error = SystemError::IoError {
            operation: "write".to_string(),
            source: "No space left on device".to_string(),
        };
        
        let main_error: Error = system_error.into();
        assert!(matches!(main_error, Error::System(_)));
    }
}

#[cfg(test)]
mod error_display_tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let error = ConfigError::LoadError {
            path: "/config.yml".to_string(),
            source: "File not found".to_string(),
        };
        
        let display = format!("{}", error);
        assert!(display.contains("config.yml"));
        assert!(display.contains("File not found"));
    }

    #[test]
    fn test_data_error_display() {
        let error = DataError::ValidationError {
            message: "Invalid data".to_string(),
            path: Some("/data.csv".to_string()),
            line: Some(42),
        };
        
        let display = format!("{}", error);
        assert!(display.contains("Invalid data"));
        assert!(display.contains("data.csv"));
        assert!(display.contains("42"));
    }

    #[test]
    fn test_processing_error_display() {
        let error = ProcessingError::StrategyError {
            strategy: "chunked".to_string(),
            message: "Memory limit exceeded".to_string(),
        };
        
        let display = format!("{}", error);
        assert!(display.contains("chunked"));
        assert!(display.contains("Memory limit exceeded"));
    }

    #[test]
    fn test_main_error_display() {
        let config_error = ConfigError::ParseError {
            message: "Invalid YAML".to_string(),
            line: Some(10),
            column: Some(5),
        };
        let main_error = Error::Config(config_error);
        
        let display = format!("{}", main_error);
        assert!(display.contains("Invalid YAML"));
        assert!(display.contains("10"));
        assert!(display.contains("5"));
    }
}

#[cfg(test)]
mod error_serialization_tests {
    use super::*;

    #[test]
    fn test_config_error_serialization() {
        let error = ConfigError::ValidationError {
            message: "Invalid survey code".to_string(),
            field: Some("survey_code".to_string()),
        };
        
        let json = serde_json::to_string(&error).expect("Should serialize");
        assert!(json.contains("Invalid survey code"));
        assert!(json.contains("survey_code"));
        
        let deserialized: ConfigError = serde_json::from_str(&json).expect("Should deserialize");
        if let ConfigError::ValidationError { message, field } = deserialized {
            assert_eq!(message, "Invalid survey code");
            assert_eq!(field, Some("survey_code".to_string()));
        }
    }

    #[test]
    fn test_main_error_serialization() {
        let data_error = DataError::FormatError {
            message: "Wrong format".to_string(),
            expected: "CSV".to_string(),
            actual: "JSON".to_string(),
        };
        let main_error = Error::Data(data_error);
        
        let json = serde_json::to_string(&main_error).expect("Should serialize");
        assert!(json.contains("Wrong format"));
        assert!(json.contains("CSV"));
        assert!(json.contains("JSON"));
        
        let deserialized: Error = serde_json::from_str(&json).expect("Should deserialize");
        assert!(matches!(deserialized, Error::Data(_)));
    }

    #[test]
    fn test_complex_error_serialization() {
        let error = ConfigError::ConfigVersionMismatch {
            file_path: "/config/survey.yml".to_string(),
            expected_version: 2,
            actual_version: 1,
        };
        
        let json = serde_json::to_string(&error).expect("Should serialize");
        let deserialized: ConfigError = serde_json::from_str(&json).expect("Should deserialize");
        
        if let ConfigError::ConfigVersionMismatch { file_path, expected_version, actual_version } = deserialized {
            assert_eq!(file_path, "/config/survey.yml");
            assert_eq!(expected_version, 2);
            assert_eq!(actual_version, 1);
        }
    }
}

#[cfg(test)]
mod error_context_tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new("Test error", "test.rs", 42, 10);
        
        assert_eq!(context.message(), "Test error");
        assert_eq!(context.file(), "test.rs");
        assert_eq!(context.line(), 42);
        assert_eq!(context.column(), 10);
    }

    #[test]
    fn test_context_ext_trait() {
        let result: Result<i32> = Err(Error::Config(ConfigError::LoadError {
            path: "/config.yml".to_string(),
            source: "Not found".to_string(),
        }));
        
        let with_context = result.with_context("Loading configuration");
        assert!(with_context.is_err());
    }

    #[test]
    fn test_option_context_ext_trait() {
        let option: Option<String> = None;
        let result = option.with_context("Expected value");
        
        assert!(result.is_err());
    }

    #[test]
    fn test_error_context_macro() {
        let context = error_context!("Test error with {} parameter", "formatted");
        assert!(context.message().contains("formatted"));
        assert!(context.file().contains("error_tests.rs"));
    }
}

#[cfg(test)]
mod error_recovery_tests {
    use super::*;

    #[test]
    fn test_retry_policy_creation() {
        let policy = RetryPolicy::new()
            .with_max_attempts(3)
            .with_initial_delay(std::time::Duration::from_millis(100))
            .with_backoff_multiplier(2.0);
        
        assert_eq!(policy.max_attempts(), 3);
        assert_eq!(policy.initial_delay(), std::time::Duration::from_millis(100));
        assert_eq!(policy.backoff_multiplier(), 2.0);
    }

    #[test]
    fn test_retry_policy_should_retry() {
        let policy = RetryPolicy::new().with_max_attempts(3);
        
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
        assert!(!policy.should_retry(4));
    }

    #[test]
    fn test_circuit_breaker_creation() {
        let breaker = CircuitBreaker::new()
            .with_failure_threshold(5)
            .with_timeout(std::time::Duration::from_secs(30));
        
        assert_eq!(breaker.failure_threshold(), 5);
        assert_eq!(breaker.timeout(), std::time::Duration::from_secs(30));
        assert!(breaker.is_closed());
    }

    #[test]
    fn test_circuit_breaker_state_transitions() {
        let mut breaker = CircuitBreaker::new().with_failure_threshold(2);
        
        // Initially closed
        assert!(breaker.is_closed());
        
        // Record failures
        breaker.record_failure();
        assert!(breaker.is_closed());
        
        breaker.record_failure();
        assert!(breaker.is_open()); // Should open after threshold
        
        // Record success should eventually close it
        breaker.record_success();
        // Note: Actual implementation might require time-based logic
    }

    #[test]
    fn test_recovery_strategy() {
        let strategy = RecoveryStrategy::new()
            .with_retry_policy(RetryPolicy::new().with_max_attempts(3))
            .with_circuit_breaker(CircuitBreaker::new().with_failure_threshold(5));
        
        assert!(strategy.retry_policy().is_some());
        assert!(strategy.circuit_breaker().is_some());
    }
}

#[cfg(test)]
mod error_telemetry_tests {
    use super::*;

    #[test]
    fn test_error_metrics_creation() {
        let metrics = ErrorMetrics::new();
        
        assert_eq!(metrics.total_errors(), 0);
        assert_eq!(metrics.error_rate(), 0.0);
        assert!(metrics.error_counts_by_type().is_empty());
    }

    #[test]
    fn test_error_metrics_recording() {
        let mut metrics = ErrorMetrics::new();
        
        let config_error = Error::Config(ConfigError::LoadError {
            path: "/config.yml".to_string(),
            source: "Not found".to_string(),
        });
        
        metrics.record_error(&config_error);
        
        assert_eq!(metrics.total_errors(), 1);
        assert!(metrics.error_counts_by_type().contains_key("Config"));
    }

    #[test]
    fn test_error_collector() {
        let mut collector = ErrorCollector::new();
        
        let error = Error::Data(DataError::ValidationError {
            message: "Invalid data".to_string(),
            path: None,
            line: None,
        });
        
        collector.collect_error(error.clone());
        
        assert_eq!(collector.error_count(), 1);
        assert!(collector.has_errors());
        
        let errors = collector.drain_errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(collector.error_count(), 0);
    }

    #[test]
    fn test_error_collector_filtering() {
        let mut collector = ErrorCollector::new();
        
        collector.collect_error(Error::Config(ConfigError::LoadError {
            path: "/config.yml".to_string(),
            source: "Not found".to_string(),
        }));
        
        collector.collect_error(Error::Data(DataError::ReadError {
            path: "/data.csv".to_string(),
            source: "Permission denied".to_string(),
        }));
        
        let config_errors = collector.errors_by_type("Config");
        assert_eq!(config_errors.len(), 1);
        
        let data_errors = collector.errors_by_type("Data");
        assert_eq!(data_errors.len(), 1);
        
        let processing_errors = collector.errors_by_type("Processing");
        assert_eq!(processing_errors.len(), 0);
    }
}

#[cfg(test)]
mod error_sanitization_tests {
    use super::*;

    #[test]
    fn test_error_sanitizer_creation() {
        let policy = SanitizationPolicy::new()
            .with_redact_paths(true)
            .with_redact_credentials(true)
            .with_max_message_length(100);
        
        let sanitizer = ErrorSanitizer::new(policy);
        assert!(sanitizer.policy().redact_paths());
        assert!(sanitizer.policy().redact_credentials());
        assert_eq!(sanitizer.policy().max_message_length(), 100);
    }

    #[test]
    fn test_error_sanitization_paths() {
        let policy = SanitizationPolicy::new().with_redact_paths(true);
        let sanitizer = ErrorSanitizer::new(policy);
        
        let error = ConfigError::LoadError {
            path: "/home/user/secret/config.yml".to_string(),
            source: "File not found".to_string(),
        };
        
        let sanitized = sanitizer.sanitize_config_error(&error);
        if let ConfigError::LoadError { path, .. } = sanitized {
            assert!(path.contains("***") || path == "[REDACTED]");
        }
    }

    #[test]
    fn test_error_sanitization_credentials() {
        let policy = SanitizationPolicy::new().with_redact_credentials(true);
        let sanitizer = ErrorSanitizer::new(policy);
        
        let error = ConfigError::ValidationError {
            message: "Invalid password: secret123".to_string(),
            field: Some("password".to_string()),
        };
        
        let sanitized = sanitizer.sanitize_config_error(&error);
        if let ConfigError::ValidationError { message, .. } = sanitized {
            assert!(!message.contains("secret123"));
            assert!(message.contains("[REDACTED]") || message.contains("***"));
        }
    }

    #[test]
    fn test_error_sanitization_message_length() {
        let policy = SanitizationPolicy::new().with_max_message_length(20);
        let sanitizer = ErrorSanitizer::new(policy);
        
        let long_message = "This is a very long error message that exceeds the limit".to_string();
        let error = DataError::ValidationError {
            message: long_message,
            path: None,
            line: None,
        };
        
        let sanitized = sanitizer.sanitize_data_error(&error);
        if let DataError::ValidationError { message, .. } = sanitized {
            assert!(message.len() <= 23); // 20 + "..."
            assert!(message.ends_with("..."));
        }
    }
}

#[cfg(test)]
mod error_localization_tests {
    use super::*;

    #[test]
    fn test_message_catalog_creation() {
        let mut catalog = MessageCatalog::new("en");
        catalog.add_message("config.load_error", "Failed to load configuration file");
        catalog.add_message("data.validation_error", "Data validation failed");
        
        assert_eq!(catalog.locale(), "en");
        assert_eq!(catalog.get_message("config.load_error"), Some("Failed to load configuration file"));
        assert_eq!(catalog.get_message("nonexistent"), None);
    }

    #[test]
    fn test_error_localizer() {
        let mut catalog = MessageCatalog::new("en");
        catalog.add_message("config.load_error", "Configuration file could not be loaded");
        
        let localizer = ErrorLocalizer::new(catalog);
        
        let error = ConfigError::LoadError {
            path: "/config.yml".to_string(),
            source: "File not found".to_string(),
        };
        
        let localized = localizer.localize_config_error(&error);
        // The exact behavior depends on implementation
        assert!(!localized.is_empty());
    }

    #[test]
    fn test_error_localizer_fallback() {
        let catalog = MessageCatalog::new("en");
        let localizer = ErrorLocalizer::new(catalog);
        
        let error = ConfigError::ValidationError {
            message: "Invalid value".to_string(),
            field: Some("test_field".to_string()),
        };
        
        let localized = localizer.localize_config_error(&error);
        // Should fall back to original message if no translation available
        assert!(localized.contains("Invalid value"));
    }

    #[test]
    fn test_message_catalog_with_parameters() {
        let mut catalog = MessageCatalog::new("en");
        catalog.add_message("config.missing_key", "Missing required key: {key} in section {section}");
        
        let params = vec![
            ("key", "database_url"),
            ("section", "runtime"),
        ];
        
        let formatted = catalog.format_message("config.missing_key", &params);
        assert!(formatted.contains("database_url"));
        assert!(formatted.contains("runtime"));
    }
}