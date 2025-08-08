//! # Error Type Definitions
//!
//! This module defines the hierarchical error type system for the Rusty BLS Data Processing system.
//! It provides specific error types for each component and operation, enabling precise error handling
//! and debugging throughout the application.
//!
//! ## Error Hierarchy
//!
//! ```text
//! Error (Main enum)
//! ├── Config(ConfigError)     - Configuration-related errors
//! ├── Data(DataError)         - Data processing and validation errors
//! ├── Processing(ProcessingError) - Data transformation and computation errors
//! ├── Output(OutputError)     - Output generation and formatting errors
//! ├── Plugin(PluginError)     - Plugin loading and execution errors
//! └── System(SystemError)     - System-level I/O and resource errors
//! ```
//!
//! ## Design Principles
//!
//! - **Specific Error Types**: Each component has its own error enum with relevant variants
//! - **Rich Context**: Errors include detailed information for debugging
//! - **Conversion Support**: Automatic conversion between error types using From trait
//! - **Serialization**: All errors support serialization for logging and telemetry
//! - **Display Implementation**: Human-readable error messages for all error types
//!
//! ## Usage
//!
//! ```rust
//! use rusty::error::{Error, ConfigError, DataError};
//!
//! // Create specific error types
//! let config_err = ConfigError::LoadError {
//!     path: "config.yml".to_string(),
//!     source: Box::new(std::io::Error::from(std::io::ErrorKind::NotFound)).to_string(),
//! };
//!
//! // Convert to main Error type
//! let main_err: Error = config_err.into();
//! ```

use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::fmt;

/// Main error type for the Rusty BLS Data Processing system
///
/// This enum represents all possible errors that can occur in the system.
/// Each variant corresponds to a specific component or operation type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    /// Configuration-related errors (loading, parsing, validation)
    Config(ConfigError),
    /// Data-related errors (reading, validation, formatting)
    Data(DataError),
    /// Processing-related errors (transformation, computation)
    Processing(ProcessingError),
    /// Output-related errors (writing, formatting, serialization)
    Output(OutputError),
    /// Plugin-related errors (loading, execution, communication)
    Plugin(PluginError),
    /// System-level errors (I/O, memory, network)
    System(SystemError),
}

/// Configuration-related error types
///
/// These errors occur during configuration loading, parsing, and validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigError {
    /// Error loading configuration file
    LoadError {
        path: String,
        source: String, // Simplified for serialization
    },
    /// Configuration validation error
    ValidationError {
        message: String,
        field: Option<String>,
    },
    /// Configuration parsing error
    ParseError {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    /// Missing required configuration
    MissingError {
        key: String,
        section: Option<String>,
    },
    /// Configuration layer is missing (e.g., overview.yml, model.yml)
    ConfigLayerMissing {
        survey_code: String,
        layer: String,
        path: String,
    },
    /// Configuration version mismatch
    ConfigVersionMismatch {
        file_path: String,
        expected_version: u32,
        actual_version: u32,
    },
    /// Error merging configuration layers
    ConfigMergeError {
        source_layer: String,
        target_layer: String,
        message: String,
    },
    /// Invalid override configuration
    InvalidOverride {
        override_path: String,
        field: String,
        message: String,
    },
    /// Unknown environment specified
    UnknownEnvironment {
        environment: String,
        valid_environments: Vec<String>,
    },
    /// Macro resolution error
    MacroResolutionError {
        macro_name: String,
        file_path: String,
        message: String,
    },
    /// DAG validation error
    DagValidationError { dag_name: String, message: String },
}

impl ConfigError {
    pub fn InvalidArgument(message: String) -> ConfigError {
        ConfigError::ValidationError {
            message,
            field: None,
        }
    }
}

impl ConfigError {
    pub(crate) fn SerializationError(message: String) -> Error {
        Error::Config(ConfigError::ParseError {
            message,
            line: None,
            column: None,
        })
    }
}

impl ConfigError {
    pub(crate) fn UnsupportedFormat(message: String) -> Error {
        Error::Config(ConfigError::ParseError {
            message,
            line: None,
            column: None,
        })
    }
}

impl ConfigError {
    pub(crate) fn MissingFile(path: String) -> Error {
        Error::Config(ConfigError::LoadError {
            path,
            source: "File not found".to_string(),
        })
    }
}

impl ConfigError {
    pub(crate) fn MergeError(message: String) -> Error {
        Error::Config(ConfigError::ConfigMergeError {
            source_layer: "unknown".to_string(),
            target_layer: "unknown".to_string(),
            message,
        })
    }
}

impl ConfigError {
    pub(crate) fn FileNotFound(path: String) -> Error {
        Error::Config(ConfigError::LoadError {
            path,
            source: "File not found".to_string(),
        })
    }
}

impl ConfigError {
    pub(crate) fn ValidationFailed(message: String) -> ConfigError {
        ConfigError::ValidationError {
            message,
            field: None,
        }
    }
}

/// Data-related error types
///
/// These errors occur during data reading, validation, and formatting operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataError {
    /// Error reading data file
    ReadError { path: String, source: String },
    /// Data validation error
    ValidationError {
        message: String,
        path: Option<String>,
        line: Option<usize>,
    },
    /// Data format error
    FormatError {
        message: String,
        expected: String,
        actual: String,
    },
    /// Schema mismatch error
    SchemaError {
        message: String,
        field: String,
        expected_type: String,
        actual_type: String,
    },
    /// Data validation error
    IoError { path: String, source: String },
}

impl DataError {
    pub(crate) fn MissingValue(p0: String) -> DataError {
        todo!()
    }
}

impl DataError {
    pub(crate) fn missing_value(message: String) -> DataError {
        DataError::ValidationError {
            message,
            path: None,
            line: None,
        }
    }

    pub(crate) fn too_many_errors(message: String) -> DataError {
        DataError::ValidationError {
            message,
            path: None,
            line: None,
        }
    }

    pub(crate) fn invalid_range(message: String) -> DataError {
        DataError::ValidationError {
            message,
            path: None,
            line: None,
        }
    }

    pub(crate) fn not_implemented(message: String) -> DataError {
        DataError::FormatError {
            message,
            expected: "implemented feature".to_string(),
            actual: "not implemented".to_string(),
        }
    }

    pub(crate) fn io_error(message: String) -> DataError {
        DataError::IoError {
            path: "unknown".to_string(),
            source: message,
        }
    }

    pub(crate) fn invalid_configuration(message: String) -> DataError {
        DataError::ValidationError {
            message,
            path: None,
            line: None,
        }
    }

    pub(crate) fn serialization_error(message: String) -> DataError {
        DataError::FormatError {
            message,
            expected: "valid serializable data".to_string(),
            actual: "invalid data".to_string(),
        }
    }

    pub(crate) fn invalid_value(message: String) -> DataError {
        DataError::ValidationError {
            message,
            path: None,
            line: None,
        }
    }

    pub(crate) fn parse_error(message: String) -> DataError {
        DataError::FormatError {
            message,
            expected: "valid data format".to_string(),
            actual: "unparseable data".to_string(),
        }
    }

    pub(crate) fn invalid_format(message: String) -> DataError {
        DataError::FormatError {
            message,
            expected: "valid format".to_string(),
            actual: "invalid format".to_string(),
        }
    }

    pub(crate) fn unsupported_format(message: String) -> DataError {
        DataError::FormatError {
            message,
            expected: "supported format".to_string(),
            actual: "unsupported format".to_string(),
        }
    }
}

/// Processing-related error types
///
/// These errors occur during data transformation and computation operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingError {
    /// Transformation error
    TransformationError { message: String, operation: String },
    /// Computation error
    ComputationError { message: String, context: String },
    /// Strategy execution error
    StrategyError { strategy: String, message: String },
    /// Pipeline stage error
    PipelineError { stage: String, message: String },
    /// Unsupported operation error
    UnsupportedOperation { operation: String, message: String },
    /// Invalid configuration error
    InvalidConfiguration(String),
    /// System error
    SystemError(String),
}

impl ProcessingError {
    /// Constructor for UnsupportedDataType variant
    pub fn UnsupportedDataType(message: String) -> ProcessingError {
        ProcessingError::UnsupportedOperation {
            operation: "data type processing".to_string(),
            message,
        }
    }

    /// Constructor for NotImplemented variant
    pub fn NotImplemented(message: String) -> ProcessingError {
        ProcessingError::UnsupportedOperation {
            operation: "not implemented".to_string(),
            message,
        }
    }
}

impl ProcessingError {
    pub(crate) fn too_many_errors(message: String) -> ProcessingError {
        ProcessingError::PipelineError {
            stage: "validation".to_string(),
            message,
        }
    }

    pub(crate) fn parse_error(message: String) -> ProcessingError {
        ProcessingError::TransformationError {
            message,
            operation: "parsing".to_string(),
        }
    }

    pub(crate) fn unsupported_data_type(message: String) -> ProcessingError {
        ProcessingError::UnsupportedOperation {
            operation: "data type processing".to_string(),
            message,
        }
    }

    pub(crate) fn io_error(message: String) -> ProcessingError {
        ProcessingError::StrategyError {
            strategy: "I/O operation".to_string(),
            message,
        }
    }

    pub(crate) fn system_error(message: String) -> ProcessingError {
        ProcessingError::StrategyError {
            strategy: "system operation".to_string(),
            message,
        }
    }

    pub(crate) fn invalid_configuration(message: String) -> Error {
        Error::Processing(ProcessingError::PipelineError {
            stage: "configuration".to_string(),
            message,
        })
    }

    pub(crate) fn resource_exhausted(message: String) -> Error {
        Error::Processing(ProcessingError::StrategyError {
            strategy: "resource management".to_string(),
            message,
        })
    }

    pub(crate) fn data_error(message: String) -> Error {
        Error::Processing(ProcessingError::TransformationError {
            message,
            operation: "data processing".to_string(),
        })
    }
}

/// Output-related error types
///
/// These errors occur during output generation, formatting, and serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputError {
    /// Error writing output file
    WriteError { path: String, source: String },
    /// Output formatting error
    FormatError { format: String, message: String },
    /// Serialization error
    SerializationError { format: String, message: String },
    /// Partitioning error
    PartitionError { strategy: String, message: String },
}

/// Plugin-related error types
///
/// These errors occur during plugin loading, execution, and communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginError {
    /// Plugin loading error
    LoadError {
        plugin: String,
        source: String,
    },
    /// Plugin execution error
    ExecutionError {
        plugin: String,
        message: String,
    },
    /// Plugin communication error
    CommunicationError {
        plugin: String,
        message: String,
    },
    /// Plugin configuration error
    ConfigurationError {
        plugin: String,
        message: String,
    },
    NotFoundError {
        plugin: String,
        message: String,
    },
}

/// System-level error types
///
/// These errors occur at the system level (I/O, memory, network operations).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemError {
    /// I/O operation error
    IoError { operation: String, source: String },
    /// Memory allocation error
    MemoryError {
        message: String,
        requested_size: Option<usize>,
    },
    /// Network operation error
    NetworkError { operation: String, source: String },
    /// Resource exhaustion error
    ResourceError { resource: String, message: String },
    /// Time-related error
    TimeError { operation: String, message: String },
    ParseError {
        format: String,
        source: String,
        context: Option<String>, // TODO: add context type
    },
}

impl SystemError {
    pub(crate) fn InvalidPath(p0: String) -> SystemError {
        SystemError::IoError {
            operation: "path validation".to_string(),
            source: format!("Invalid path: {p0}"),
        }
    }
}

impl SystemError {
    pub(crate) fn invalid_path(path: String) -> SystemError {
        SystemError::IoError {
            operation: "path validation".to_string(),
            source: format!("Invalid path: {path}"),
        }
    }

    pub(crate) fn file_not_found(path: String) -> SystemError {
        SystemError::IoError {
            operation: "file access".to_string(),
            source: format!("File not found: {path}"),
        }
    }
}

// Type alias for Result with our Error type
pub type Result<T> = std::result::Result<T, Error>;

// Implementation of Display trait for all error types
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Config(e) => write!(f, "Configuration error: {e}"),
            Error::Data(e) => write!(f, "Data error: {e}"),
            Error::Processing(e) => write!(f, "Processing error: {e}"),
            Error::Output(e) => write!(f, "Output error: {e}"),
            Error::Plugin(e) => write!(f, "Plugin error: {e}"),
            Error::System(e) => write!(f, "System error: {e}"),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::LoadError { path, source } => {
                write!(f, "Failed to load config from '{path}': {source}")
            }
            ConfigError::ValidationError { message, field } => {
                if let Some(field) = field {
                    write!(f, "Validation error in field '{field}': {message}")
                } else {
                    write!(f, "Validation error: {message}")
                }
            }
            ConfigError::ParseError {
                message,
                line,
                column,
            } => {
                if let (Some(line), Some(column)) = (line, column) {
                    write!(
                        f,
                        "Parse error at line {line}, column {column}: {message}"
                    )
                } else {
                    write!(f, "Parse error: {message}")
                }
            }
            ConfigError::MissingError { key, section } => {
                if let Some(section) = section {
                    write!(
                        f,
                        "Missing required configuration key '{key}' in section '{section}'"
                    )
                } else {
                    write!(f, "Missing required configuration key '{key}'")
                }
            }
            ConfigError::ConfigLayerMissing {
                survey_code,
                layer,
                path,
            } => {
                write!(
                    f,
                    "Missing configuration layer '{layer}' for survey '{survey_code}' at path '{path}'"
                )
            }
            ConfigError::ConfigVersionMismatch {
                file_path,
                expected_version,
                actual_version,
            } => {
                write!(
                    f,
                    "Version mismatch in '{file_path}': expected {expected_version}, found {actual_version}"
                )
            }
            ConfigError::ConfigMergeError {
                source_layer,
                target_layer,
                message,
            } => {
                write!(
                    f,
                    "Failed to merge '{source_layer}' into '{target_layer}': {message}"
                )
            }
            ConfigError::InvalidOverride {
                override_path,
                field,
                message,
            } => {
                write!(
                    f,
                    "Invalid override in '{override_path}' for field '{field}': {message}"
                )
            }
            ConfigError::UnknownEnvironment {
                environment,
                valid_environments,
            } => {
                write!(
                    f,
                    "Unknown environment '{}'. Valid environments: {}",
                    environment,
                    valid_environments.join(", ")
                )
            }
            ConfigError::MacroResolutionError {
                macro_name,
                file_path,
                message,
            } => {
                write!(
                    f,
                    "Failed to resolve macro '{macro_name}' in '{file_path}': {message}"
                )
            }
            ConfigError::DagValidationError { dag_name, message } => {
                write!(f, "DAG validation error in '{dag_name}': {message}")
            }
        }
    }
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::ReadError { path, source } => {
                write!(f, "Failed to read data from '{path}': {source}")
            }
            DataError::ValidationError {
                message,
                path,
                line,
            } => match (path, line) {
                (Some(path), Some(line)) => write!(
                    f,
                    "Validation error in '{path}' at line {line}: {message}"
                ),
                (Some(path), None) => write!(f, "Validation error in '{path}': {message}"),
                _ => write!(f, "Validation error: {message}"),
            },
            DataError::FormatError {
                message,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Format error: {message}. Expected: {expected}, Actual: {actual}"
                )
            }
            DataError::SchemaError {
                message,
                field,
                expected_type,
                actual_type,
            } => {
                write!(
                    f,
                    "Schema error in field '{field}': {message}. Expected: {expected_type}, Actual: {actual_type}"
                )
            }
            DataError::IoError { path, source } => {
                write!(f, "I/O error at path '{path}': {source}")
            }
        }
    }
}

impl fmt::Display for ProcessingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessingError::TransformationError { message, operation } => {
                write!(
                    f,
                    "Transformation error in operation '{operation}': {message}"
                )
            }
            ProcessingError::ComputationError { message, context } => {
                write!(f, "Computation error in context '{context}': {message}")
            }
            ProcessingError::StrategyError { strategy, message } => {
                write!(f, "Strategy error in '{strategy}': {message}")
            }
            ProcessingError::PipelineError { stage, message } => {
                write!(f, "Pipeline error in stage '{stage}': {message}")
            }
            ProcessingError::UnsupportedOperation { operation, message } => {
                write!(f, "Unsupported operation '{operation}': {message}")
            }
            ProcessingError::InvalidConfiguration(message) => {
                write!(f, "Invalid configuration: {message}")
            }
            ProcessingError::SystemError(message) => {
                write!(f, "System error: {message}")
            }
        }
    }
}

impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputError::WriteError { path, source } => {
                write!(f, "Failed to write output to '{path}': {source}")
            }
            OutputError::FormatError { format, message } => {
                write!(f, "Format error in '{format}': {message}")
            }
            OutputError::SerializationError { format, message } => {
                write!(f, "Serialization error in '{format}': {message}")
            }
            OutputError::PartitionError { strategy, message } => write!(
                f,
                "Partition error with strategy '{strategy}': {message}"
            ),
        }
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::LoadError { plugin, source } => {
                write!(f, "Failed to load plugin '{plugin}': {source}")
            }
            PluginError::ExecutionError { plugin, message } => {
                write!(f, "Execution error in plugin '{plugin}': {message}")
            }
            PluginError::CommunicationError { plugin, message } => write!(
                f,
                "Communication error with plugin '{plugin}': {message}"
            ),
            PluginError::ConfigurationError { plugin, message } => {
                write!(f, "Configuration error in plugin '{plugin}': {message}")
            }
            PluginError::NotFoundError { plugin, message } => {
                write!(f, "Plugin '{plugin}' not found: {message}")
            }
        }
    }
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemError::IoError { operation, source } => {
                write!(f, "I/O error during '{operation}': {source}")
            }
            SystemError::MemoryError {
                message,
                requested_size,
            } => {
                if let Some(size) = requested_size {
                    write!(f, "Memory error (requested {size} bytes): {message}")
                } else {
                    write!(f, "Memory error: {message}")
                }
            }
            SystemError::NetworkError { operation, source } => {
                write!(f, "Network error during '{operation}': {source}")
            }
            SystemError::ResourceError { resource, message } => {
                write!(f, "Resource error with '{resource}': {message}")
            }
            SystemError::TimeError { operation, message } => {
                write!(f, "Time error during '{operation}': {message}")
            }
            SystemError::ParseError {
                format,
                source,
                context,
            } => {
                if let Some(ctx) = context {
                    write!(
                        f,
                        "Parse error in format '{format}' with context '{ctx}': {source}"
                    )
                } else {
                    write!(f, "Parse error in format '{format}': {source}")
                }
            }
        }
    }
}

// Implementation of std::error::Error trait
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        None // Simplified implementation
    }
}

impl StdError for ConfigError {}
impl StdError for DataError {}
impl StdError for ProcessingError {}
impl StdError for OutputError {}
impl StdError for PluginError {}
impl StdError for SystemError {}

// Implementation of From traits for error conversion
impl From<ConfigError> for Error {
    fn from(err: ConfigError) -> Self {
        Error::Config(err)
    }
}

impl From<DataError> for Error {
    fn from(err: DataError) -> Self {
        Error::Data(err)
    }
}

impl From<ProcessingError> for Error {
    fn from(err: ProcessingError) -> Self {
        Error::Processing(err)
    }
}

impl From<OutputError> for Error {
    fn from(err: OutputError) -> Self {
        Error::Output(err)
    }
}

impl From<PluginError> for Error {
    fn from(err: PluginError) -> Self {
        Error::Plugin(err)
    }
}

impl From<SystemError> for Error {
    fn from(err: SystemError) -> Self {
        Error::System(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::System(SystemError::IoError {
            operation: "I/O operation".to_string(),
            source: err.to_string(),
        })
    }
}
