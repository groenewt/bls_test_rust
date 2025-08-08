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
pub mod file;
pub mod format;
pub mod path;
pub mod time;
pub mod validation;

// Re-export commonly used functions and types
pub use file::{FileUtils, read_to_string, write_string};
pub use format::{FormatUtils, format_bls_value, format_survey_code, parse_bls_value};
pub use path::{PathUtils, ensure_directory, get_config_path, get_data_path, get_output_path};
pub use time::{TimeUtils, current_timestamp, format_timestamp, parse_timestamp};
pub use validation::{ValidationUtils, validate_config, validate_file_path, validate_survey_code};

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
