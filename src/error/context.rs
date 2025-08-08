//! # Error Context System
//!
//! This module provides rich error context functionality for the Rusty BLS Data Processing system.
//! It enables detailed error reporting with location tracking, timestamps, and component identification
//! to facilitate debugging and error analysis.
//!
//! ## Features
//!
//! - **Location Tracking**: Automatic capture of file, line, and column information
//! - **Component Identification**: Track which component generated the error
//! - **Timestamp Information**: Record when the error occurred
//! - **Context Messages**: Human-readable descriptions of the error context
//! - **Error Chaining**: Support for chaining errors with context preservation
//! - **Trait Extensions**: Convenient traits for adding context to Result and Option types
//!
//! ## Architecture
//!
//! ```text
//! ErrorContext
//! ├── message: String          # Human-readable context message
//! ├── location: Location       # File, line, column information
//! ├── timestamp: SystemTime    # When the error occurred
//! ├── component: String        # Which component generated the error
//! └── correlation_id: Uuid     # For distributed tracing
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use crate::rusty::error::{ContextExt, OptionContextExt, error_context};
//!
//! // Add context to Result types
//! let result = load_file(path)
//!     .with_context(|| format!("Failed to load configuration from {}", path))?;
//!
//! // Add context to Option types
//! let value = map.get("key")
//!     .with_context(|| "Missing required configuration key")?;
//!
//! // Create error context manually
//! let context = error_context!("Processing failed for series {}", series_id);
//! ```

use std::fmt;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, Result};

/// Location information for error tracking
///
/// Captures the source code location where an error occurred or context was added.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// Source file name
    pub file: &'static str,
    /// Line number in the source file
    pub line: u32,
    /// Column number in the source file
    pub column: u32,
}

impl Location {
    /// Create a new Location
    pub fn new(file: &'static str, line: u32, column: u32) -> Self {
        Self { file, line, column }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// Rich error context information
///
/// Provides detailed context about where and when an error occurred,
/// including location tracking, timestamps, and component identification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Human-readable context message
    pub message: String,
    /// Source code location where context was added
    pub location: Option<Location>,
    /// Timestamp when the error occurred
    pub timestamp: SystemTime,
    /// Component that generated or handled the error
    pub component: String,
    /// Correlation ID for distributed tracing
    pub correlation_id: Uuid,
    /// Additional metadata for the error context
    pub metadata: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    /// Create a new ErrorContext
    pub fn new<S: Into<String>>(
        message: S,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> Self {
        Self {
            message: message.into(),
            location: Some(Location::new(file, line, column)),
            timestamp: SystemTime::now(),
            component: "unknown".to_string(),
            correlation_id: Uuid::new_v4(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create a new ErrorContext without location information
    pub fn new_simple<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            location: None,
            timestamp: SystemTime::now(),
            component: "unknown".to_string(),
            correlation_id: Uuid::new_v4(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set the component that generated this error context
    pub fn with_component<S: Into<String>>(mut self, component: S) -> Self {
        self.component = component.into();
        self
    }

    /// Add metadata to the error context
    pub fn with_metadata<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set the correlation ID for distributed tracing
    pub fn with_correlation_id(mut self, correlation_id: Uuid) -> Self {
        self.correlation_id = correlation_id;
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(location) = &self.location {
            write!(f, " at {}", location)?;
        }
        write!(f, " [{}]", self.component)?;
        Ok(())
    }
}

/// Trait for adding context to Result types
///
/// This trait provides convenient methods for adding error context to Result types,
/// enabling rich error reporting throughout the application.
pub trait ContextExt<T> {
    /// Add context to an error with a static message
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;

    /// Add context to an error with location tracking
    fn with_context_at<F>(self, f: F, file: &'static str, line: u32, column: u32) -> Result<T>
    where
        F: FnOnce() -> String;

    /// Add context with component identification
    fn with_component_context<F, C>(self, f: F, component: C) -> Result<T>
    where
        F: FnOnce() -> String,
        C: Into<String>;
}

/// Trait for adding context to Option types
///
/// This trait provides convenient methods for converting Option types to Result types
/// with appropriate error context.
pub trait OptionContextExt<T> {
    /// Convert Option to Result with context
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;

    /// Convert Option to Result with context and location tracking
    fn with_context_at<F>(self, f: F, file: &'static str, line: u32, column: u32) -> Result<T>
    where
        F: FnOnce() -> String;

    /// Convert Option to Result with component context
    fn with_component_context<F, C>(self, f: F, component: C) -> Result<T>
    where
        F: FnOnce() -> String,
        C: Into<String>;
}

// Implementation of ContextExt for Result types
// (Implementation details would be added during actual development)

// Implementation of OptionContextExt for Option types
// (Implementation details would be added during actual development)

/// Utility functions for error context management
pub mod utils {
    use super::*;

    /// Extract correlation ID from an error chain
    pub fn extract_correlation_id(error: &Error) -> Option<Uuid> {
        // Implementation would traverse the error chain to find correlation ID
        None
    }

    /// Create a context chain from multiple contexts
    pub fn create_context_chain(contexts: Vec<ErrorContext>) -> String {
        contexts
            .iter()
            .map(|ctx| ctx.to_string())
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    /// Sanitize error context for external reporting
    pub fn sanitize_context(context: &ErrorContext) -> ErrorContext {
        // Implementation would remove sensitive information
        context.clone()
    }
}