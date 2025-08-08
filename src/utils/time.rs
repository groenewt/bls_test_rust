//! # Time Utilities
//!
//! This module provides time formatting and parsing utilities for the Rusty BLS Data Processing system.

use chrono::{DateTime, Utc, NaiveDateTime, TimeZone};
use crate::error::{Result, SystemError};

/// Time utilities for the BLS data processing system
pub struct TimeUtils;

impl TimeUtils {
    /// Create a new TimeUtils instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for TimeUtils {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current timestamp as Unix seconds
pub fn current_timestamp() -> u64 {
    Utc::now().timestamp() as u64
}

/// Format timestamp to ISO 8601 string
pub fn format_timestamp(timestamp: u64) -> Result<String> {
    let dt = Utc.timestamp_opt(timestamp as i64, 0)
        .single()
        .ok_or_else(|| crate::error::Error::System(SystemError::TimeError {
            operation: "format_timestamp".to_string(),
            message: format!("Invalid timestamp: {}", timestamp),
        }))?;
    
    Ok(dt.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// Parse ISO 8601 string to timestamp
pub fn parse_timestamp(timestamp_str: &str) -> Result<u64> {
    let dt = DateTime::parse_from_rfc3339(timestamp_str)
        .map_err(|e| crate::error::Error::System(SystemError::TimeError {
            operation: "parse_timestamp".to_string(),
            message: e.to_string(),
        }))?;
    
    Ok(dt.timestamp() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
    }

    #[test]
    fn test_format_timestamp() {
        let result = format_timestamp(1609459200); // 2021-01-01 00:00:00 UTC
        assert!(result.is_ok());
        assert!(result.unwrap().starts_with("2021-01-01"));
    }

    #[test]
    fn test_parse_timestamp() {
        let result = parse_timestamp("2021-01-01T00:00:00Z");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1609459200);
    }
}