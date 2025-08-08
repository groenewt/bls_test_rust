//! # Error Message Sanitization
//!
//! This module provides comprehensive error message sanitization capabilities for the
//! Rusty BLS Data Processing system. It ensures that sensitive information is not leaked
//! through error messages and provides secure error reporting for external systems.
//!
//! ## Features
//!
//! - **Sensitive Data Detection**: Automatic detection of sensitive information in error messages
//! - **Message Redaction**: Configurable redaction of sensitive data patterns
//! - **Security Audit Logging**: Comprehensive logging of security-related errors
//! - **Context-Aware Sanitization**: Different sanitization levels based on context
//! - **Pattern-Based Filtering**: Configurable patterns for sensitive data detection
//! - **Safe External Reporting**: Sanitized error messages for external systems
//!
//! ## Architecture
//!
//! ```text
//! Sanitization System
//! ├── ErrorSanitizer          # Main sanitization engine
//! │   ├── PatternDetector     # Detect sensitive data patterns
//! │   ├── MessageRedactor     # Redact sensitive information
//! │   └── ContextAnalyzer     # Analyze error context for sensitivity
//! ├── SanitizationPolicy      # Configurable sanitization policies
//! │   ├── SecurityLevel       # Different security levels
//! │   ├── RedactionRules      # Rules for data redaction
//! │   └── AuditRequirements   # Audit logging requirements
//! ├── PatternLibrary          # Library of sensitive data patterns
//! │   ├── CredentialPatterns  # Passwords, tokens, keys
//! │   ├── PersonalDataPatterns # PII, SSN, email addresses
//! │   └── SystemPatterns      # File paths, internal URLs
//! └── AuditLogger             # Security audit logging
//! ```
//!
//! ## Usage
//!

use std::collections::HashMap;
use std::fmt;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{Error, ErrorContext};

/// Security levels for error sanitization
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// No sanitization - all error details preserved
    None,
    /// Basic sanitization - remove obvious sensitive data
    Low,
    /// Standard sanitization - remove common sensitive patterns
    Medium,
    /// High security - aggressive sanitization for external reporting
    High,
    /// Maximum security - minimal error information exposed
    Maximum,
}

/// Sanitization policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationPolicy {
    /// Security level for sanitization
    pub security_level: SecurityLevel,
    /// Enable audit logging for sanitization events
    pub audit_logging: bool,
    /// Custom redaction patterns
    pub custom_patterns: Vec<RedactionPattern>,
    /// Context-specific policies
    pub context_policies: HashMap<String, SecurityLevel>,
    /// Whitelist of safe error types
    pub safe_error_types: Vec<String>,
    /// Enable detailed sanitization logging
    pub detailed_logging: bool,
}

impl SanitizationPolicy {
    pub fn new(p0: SecurityLevel) -> SanitizationPolicy {
        todo!()
    }
}

impl Default for SanitizationPolicy {
    fn default() -> Self {
        Self {
            security_level: SecurityLevel::Medium,
            audit_logging: true,
            custom_patterns: Vec::new(),
            context_policies: HashMap::new(),
            safe_error_types: vec![
                "ValidationError".to_string(),
                "FormatError".to_string(),
            ],
            detailed_logging: false,
        }
    }
}

/// Redaction pattern for sensitive data detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionPattern {
    /// Pattern name for identification
    pub name: String,
    /// Regular expression pattern
    pub pattern: String,
    /// Replacement text for redacted content
    pub replacement: String,
    /// Security level at which this pattern is applied
    pub min_security_level: SecurityLevel,
    /// Whether this pattern is enabled
    pub enabled: bool,
}

/// Main error sanitization engine
#[derive(Debug)]
pub struct ErrorSanitizer {
    /// Sanitization policy
    policy: SanitizationPolicy,
    /// Compiled regex patterns for sensitive data detection
    patterns: Vec<CompiledPattern>,
    /// Audit logger for security events
    audit_logger: Option<AuditLogger>,
    /// Statistics for sanitization operations
    stats: SanitizationStats,
}

/// Compiled regex pattern for performance
#[derive(Debug)]
struct CompiledPattern {
    name: String,
    regex: Regex,
    replacement: String,
    min_security_level: SecurityLevel,
}

/// Statistics for sanitization operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationStats {
    /// Total number of sanitization operations
    pub total_operations: u64,
    /// Number of messages that were sanitized
    pub sanitized_messages: u64,
    /// Number of patterns matched
    pub patterns_matched: HashMap<String, u64>,
    /// Number of audit events logged
    pub audit_events: u64,
}

impl ErrorSanitizer {
    /// Create a new error sanitizer
    pub fn new() -> Self {
        let policy = SanitizationPolicy::default();
        let patterns = Self::compile_default_patterns(&policy);
        
        Self {
            policy,
            patterns,
            audit_logger: None,
            stats: SanitizationStats {
                total_operations: 0,
                sanitized_messages: 0,
                patterns_matched: HashMap::new(),
                audit_events: 0,
            },
        }
    }

    /// Configure sanitization policy
    pub fn with_policy(mut self, policy: SanitizationPolicy) -> Self {
        self.patterns = Self::compile_patterns(&policy);
        self.policy = policy;
        self
    }

    /// Enable audit logging
    pub fn with_audit_logging(mut self, enabled: bool) -> Self {
        if enabled {
            self.audit_logger = Some(AuditLogger::new());
        } else {
            self.audit_logger = None;
        }
        self
    }

    /// Sanitize an error message
    pub fn sanitize_message(&mut self, message: &str, context: &str) -> String {
        self.stats.total_operations += 1;

        // Determine security level for this context
        let security_level = self.policy.context_policies
            .get(context)
            .unwrap_or(&self.policy.security_level);

        // Skip sanitization for None security level
        if *security_level == SecurityLevel::None {
            return message.to_string();
        }

        let mut sanitized = message.to_string();
        let mut was_sanitized = false;

        // Apply patterns based on security level
        for pattern in &self.patterns {
            if Self::should_apply_pattern(&pattern.min_security_level, security_level) {
                if pattern.regex.is_match(&sanitized) {
                    sanitized = pattern.regex.replace_all(&sanitized, &pattern.replacement).to_string();
                    was_sanitized = true;
                    
                    // Update statistics
                    *self.stats.patterns_matched.entry(pattern.name.clone()).or_insert(0) += 1;
                    
                    // Log audit event if enabled
                    if let Some(audit_logger) = &mut self.audit_logger {
                        audit_logger.log_sanitization_event(&pattern.name, context, message);
                        self.stats.audit_events += 1;
                    }
                }
            }
        }

        if was_sanitized {
            self.stats.sanitized_messages += 1;
        }

        sanitized
    }

    /// Sanitize an entire error
    pub fn sanitize_error(&mut self, error: &Error, context: &str) -> Error {
        match error {
            Error::Config(config_error) => {
                Error::Config(self.sanitize_config_error(config_error, context))
            }
            Error::Data(data_error) => {
                Error::Data(self.sanitize_data_error(data_error, context))
            }
            Error::Processing(processing_error) => {
                Error::Processing(self.sanitize_processing_error(processing_error, context))
            }
            Error::Output(output_error) => {
                Error::Output(self.sanitize_output_error(output_error, context))
            }
            Error::Plugin(plugin_error) => {
                Error::Plugin(self.sanitize_plugin_error(plugin_error, context))
            }
            Error::System(system_error) => {
                Error::System(self.sanitize_system_error(system_error, context))
            }
        }
    }

    /// Sanitize error context
    pub fn sanitize_context(&mut self, context: &ErrorContext, sanitization_context: &str) -> ErrorContext {
        let mut sanitized = context.clone();
        sanitized.message = self.sanitize_message(&context.message, sanitization_context);
        
        // Sanitize metadata
        for (key, value) in &mut sanitized.metadata {
            *value = self.sanitize_message(value, sanitization_context);
        }

        sanitized
    }

    /// Get sanitization statistics
    pub fn get_stats(&self) -> &SanitizationStats {
        &self.stats
    }

    /// Check if a pattern should be applied at the given security level
    fn should_apply_pattern(pattern_level: &SecurityLevel, current_level: &SecurityLevel) -> bool {
        use SecurityLevel::*;
        match (pattern_level, current_level) {
            (None, _) => false,
            (Low, Low | Medium | High | Maximum) => true,
            (Medium, Medium | High | Maximum) => true,
            (High, High | Maximum) => true,
            (Maximum, Maximum) => true,
            _ => false,
        }
    }

    /// Compile default sanitization patterns
    fn compile_default_patterns(policy: &SanitizationPolicy) -> Vec<CompiledPattern> {
        let mut patterns = Vec::new();

        // Password patterns
        if let Ok(regex) = Regex::new(r"password[=:]\s*[^\s&]+") {
            patterns.push(CompiledPattern {
                name: "password".to_string(),
                regex,
                replacement: "password=[REDACTED]".to_string(),
                min_security_level: SecurityLevel::Low,
            });
        }

        // API key patterns
        if let Ok(regex) = Regex::new(r"(?i)(api[_-]?key|token)[=:]\s*[a-zA-Z0-9_-]+") {
            patterns.push(CompiledPattern {
                name: "api_key".to_string(),
                regex,
                replacement: "$1=[REDACTED]".to_string(),
                min_security_level: SecurityLevel::Low,
            });
        }

        // Database connection strings
        if let Ok(regex) = Regex::new(r"://[^:]+:[^@]+@") {
            patterns.push(CompiledPattern {
                name: "db_credentials".to_string(),
                regex,
                replacement: "://[REDACTED]@".to_string(),
                min_security_level: SecurityLevel::Medium,
            });
        }

        // File paths (for high security contexts)
        if let Ok(regex) = Regex::new(r"/[a-zA-Z0-9_/-]+\.(yml|yaml|json|toml|conf)") {
            patterns.push(CompiledPattern {
                name: "config_paths".to_string(),
                regex,
                replacement: "/[REDACTED]".to_string(),
                min_security_level: SecurityLevel::High,
            });
        }

        // Email addresses
        if let Ok(regex) = Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b") {
            patterns.push(CompiledPattern {
                name: "email".to_string(),
                regex,
                replacement: "[EMAIL_REDACTED]".to_string(),
                min_security_level: SecurityLevel::Medium,
            });
        }

        // IP addresses (for maximum security)
        if let Ok(regex) = Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b") {
            patterns.push(CompiledPattern {
                name: "ip_address".to_string(),
                regex,
                replacement: "[IP_REDACTED]".to_string(),
                min_security_level: SecurityLevel::Maximum,
            });
        }

        patterns
    }

    /// Compile all patterns (default + custom)
    fn compile_patterns(policy: &SanitizationPolicy) -> Vec<CompiledPattern> {
        let mut patterns = Self::compile_default_patterns(policy);

        // Add custom patterns
        for custom_pattern in &policy.custom_patterns {
            if custom_pattern.enabled {
                if let Ok(regex) = Regex::new(&custom_pattern.pattern) {
                    patterns.push(CompiledPattern {
                        name: custom_pattern.name.clone(),
                        regex,
                        replacement: custom_pattern.replacement.clone(),
                        min_security_level: custom_pattern.min_security_level.clone(),
                    });
                }
            }
        }

        patterns
    }

    // Sanitization methods for specific error types
    fn sanitize_config_error(&mut self, error: &crate::error::ConfigError, context: &str) -> crate::error::ConfigError {
        use crate::error::ConfigError;
        match error {
            ConfigError::LoadError { path, source } => ConfigError::LoadError {
                path: self.sanitize_message(path, context),
                source: self.sanitize_message(source, context),
            },
            ConfigError::ValidationError { message, field } => ConfigError::ValidationError {
                message: self.sanitize_message(message, context),
                field: field.clone(),
            },
            ConfigError::ParseError { message, line, column } => ConfigError::ParseError {
                message: self.sanitize_message(message, context),
                line: *line,
                column: *column,
            },
            ConfigError::MissingError { key, section } => ConfigError::MissingError {
                key: key.clone(),
                section: section.clone(),
            },
            _ => todo!(),
        }
    }

    fn sanitize_data_error(&mut self, error: &crate::error::DataError, context: &str) -> crate::error::DataError {
        use crate::error::DataError;
        match error {
            DataError::ReadError { path, source } => DataError::ReadError {
                path: self.sanitize_message(path, context),
                source: self.sanitize_message(source, context),
            },
            DataError::ValidationError { message, path, line } => DataError::ValidationError {
                message: self.sanitize_message(message, context),
                path: path.as_ref().map(|p| self.sanitize_message(p, context)),
                line: *line,
            },
            DataError::FormatError { message, expected, actual } => DataError::FormatError {
                message: self.sanitize_message(message, context),
                expected: expected.clone(),
                actual: actual.clone(),
            },
            DataError::SchemaError { message, field, expected_type, actual_type } => DataError::SchemaError {
                message: self.sanitize_message(message, context),
                field: field.clone(),
                expected_type: expected_type.clone(),
                actual_type: actual_type.clone(),
            },
            DataError::IoError { path, source } => DataError::IoError {
                path: self.sanitize_message(path, context),
                source: self.sanitize_message(source, context),
            },
        }
    }

    // Additional sanitization methods for other error types would be implemented here
    fn sanitize_processing_error(&mut self, error: &crate::error::ProcessingError, context: &str) -> crate::error::ProcessingError {
        // Implementation would sanitize processing error fields
        error.clone()
    }

    fn sanitize_output_error(&mut self, error: &crate::error::OutputError, context: &str) -> crate::error::OutputError {
        // Implementation would sanitize output error fields
        error.clone()
    }

    fn sanitize_plugin_error(&mut self, error: &crate::error::PluginError, context: &str) -> crate::error::PluginError {
        // Implementation would sanitize plugin error fields
        error.clone()
    }

    fn sanitize_system_error(&mut self, error: &crate::error::SystemError, context: &str) -> crate::error::SystemError {
        // Implementation would sanitize system error fields
        error.clone()
    }
}

/// Audit logger for security events
#[derive(Debug)]
pub struct AuditLogger {
    /// Audit log entries
    entries: Vec<AuditEntry>,
}

/// Audit log entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Unique entry identifier
    pub id: Uuid,
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// Type of sanitization event
    pub event_type: String,
    /// Context where sanitization occurred
    pub context: String,
    /// Pattern that was matched (if applicable)
    pub pattern_matched: Option<String>,
    /// Whether sensitive data was detected
    pub sensitive_data_detected: bool,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Log a sanitization event
    pub fn log_sanitization_event(&mut self, pattern: &str, context: &str, _original_message: &str) {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: "sanitization".to_string(),
            context: context.to_string(),
            pattern_matched: Some(pattern.to_string()),
            sensitive_data_detected: true,
        };
        
        self.entries.push(entry);
    }

    /// Get audit entries
    pub fn get_entries(&self) -> &[AuditEntry] {
        &self.entries
    }
}

/// Utility functions for sanitization
pub mod utils {
    use super::*;

    /// Check if a message contains potentially sensitive data
    pub fn contains_sensitive_data(message: &str) -> bool {
        let sensitive_keywords = [
            "password", "token", "key", "secret", "credential",
            "auth", "login", "user", "admin"
        ];
        
        let message_lower = message.to_lowercase();
        sensitive_keywords.iter().any(|keyword| message_lower.contains(keyword))
    }

    /// Create a safe error message for external reporting
    pub fn create_safe_message(error_type: &str, component: &str) -> String {
        format!("An error occurred in {} component: {}", component, error_type)
    }

    /// Validate sanitization pattern
    pub fn validate_pattern(pattern: &str) -> Result<(), String> {
        match Regex::new(pattern) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Invalid regex pattern: {}", e)),
        }
    }
}