//! # Error Handling Module
//!
//! This module provides comprehensive error management for the Rusty BLS Data Processing system.
//! It implements an enterprise-level error handling system with the following features:
//!
//! - **Hierarchical Error Types**: Specific error types for each component
//! - **Rich Error Context**: Detailed error reporting with location tracking
//! - **Error Recovery**: Automatic retry and circuit breaker patterns
//! - **Security Features**: Error sanitization and audit logging
//! - **Telemetry**: Error metrics and monitoring integration
//! - **Internationalization**: Multi-language error message support
//!
//! ## Architecture
//!
//! ```text
//! src/error/
//! ├── mod.rs              # Module entry point and re-exports
//! ├── types.rs            # Error type definitions and hierarchy
//! ├── context.rs          # Error context functionality and traits
//! ├── recovery.rs         # Error recovery mechanisms and retry logic
//! ├── telemetry.rs        # Error metrics and monitoring
//! ├── sanitization.rs     # Error message sanitization for security
//! └── i18n.rs            # Internationalization support for error messages
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use crate::rusty::error::{Error, Result, ContextExt};
//!
//! fn process_data(input: &str) -> Result<String> {
//!     // Implementation with proper error handling
//!     Ok(format!("Processed: {}", input))
//! }
//! ```

// Re-export core error types and utilities
pub use context::{ContextExt, ErrorContext, OptionContextExt};
pub use types::{ConfigError, DataError, OutputError, PluginError, ProcessingError, SystemError};
pub use types::{Error, Result};

// Re-export enterprise features
pub use i18n::{ErrorLocalizer, MessageCatalog};
pub use recovery::{CircuitBreaker, RecoveryStrategy, RetryPolicy};
pub use sanitization::{ErrorSanitizer, SanitizationPolicy};
pub use telemetry::{ErrorCollector, ErrorMetrics};

// Module declarations
pub mod context;
pub mod i18n;
pub mod recovery;
pub mod sanitization;
pub mod telemetry;
pub mod types;

// Convenience macros
#[macro_export]
macro_rules! error_context {
    ($msg:expr) => {
        $crate::error::ErrorContext::new($msg, file!(), line!(), column!())
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::error::ErrorContext::new(format!($fmt, $($arg)*), file!(), line!(), column!())
    };
}

// Type aliases for convenience
pub type ConfigResult<T> = std::result::Result<T, ConfigError>;
pub type DataResult<T> = std::result::Result<T, DataError>;
pub type ProcessingResult<T> = std::result::Result<T, ProcessingError>;
pub type OutputResult<T> = std::result::Result<T, OutputError>;
pub type PluginResult<T> = std::result::Result<T, PluginError>;
pub type SystemResult<T> = std::result::Result<T, SystemError>;
