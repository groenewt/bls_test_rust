//! # Utility Module
//!
//! This module provides common utility functions and helpers used throughout the Rusty BLS Data Processing system.
//! 
//! ## Components
//! 
//! - [`path`]: Path manipulation and validation utilities
//! - [`file`]: File I/O operations and helpers
//! - [`time`]: Time formatting and parsing utilities
//! - [`format`]: Data formatting and conversion utilities
//! - [`validation`]: Data validation helpers and rules
//! 
//! ## Usage

// Module declarations
pub mod path;
pub mod file;
pub mod time;
pub mod format;
pub mod validation;

// Re-export commonly used functions and types
pub use path::{PathUtils, get_config_path, get_data_path, get_output_path, ensure_directory};
pub use file::{FileUtils, read_to_string, write_string};
pub use time::{TimeUtils, current_timestamp, format_timestamp, parse_timestamp};
pub use format::{FormatUtils, format_bls_value, parse_bls_value, format_survey_code};
pub use validation::{ValidationUtils, validate_survey_code, validate_file_path, validate_config};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all modules are accessible
        // This is a compile-time test to ensure all modules are properly exported
        let _ = path::PathUtils::new();
        let _ = file::FileUtils::new();
        let _ = time::TimeUtils::new();
        let _ = format::FormatUtils::new();
        let _ = validation::ValidationUtils::new();
    }
}