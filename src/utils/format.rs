//! # Format Utilities
//!
//! This module provides data formatting and conversion utilities for the Rusty BLS Data Processing system.
//! It handles BLS-specific formatting requirements, data type conversions, and standardization.
//!
//! ## Features
//!
//! - BLS value formatting and parsing
//! - Survey code normalization
//! - Data type conversions
//! - Number formatting with precision control
//! - String sanitization and normalization
//!
//! ## Usage

use crate::error::{DataError, Result};
use regex::Regex;

/// Format utilities for the BLS data processing system
pub struct FormatUtils {
    // Compiled regex patterns for performance
    survey_code_pattern: Regex,
    numeric_pattern: Regex,
}

impl FormatUtils {
    /// Create a new FormatUtils instance
    pub fn new() -> Self {
        Self {
            survey_code_pattern: Regex::new(r"^[A-Za-z]{2}$").unwrap(),
            numeric_pattern: Regex::new(r"^-?\d+\.?\d*$").unwrap(),
        }
    }

    /// Validate and format a survey code
    pub fn format_survey_code(&self, code: &str) -> Result<String> {
        let trimmed = code.trim().to_uppercase();

        if !self.survey_code_pattern.is_match(&trimmed) {
            return Err(crate::error::Error::Data(DataError::invalid_format(
                format!("Invalid survey code format: {code}"),
            )));
        }

        Ok(trimmed)
    }

    /// Parse and validate a BLS numeric value
    pub fn parse_bls_value(&self, value_str: &str) -> Result<f64> {
        let trimmed = value_str.trim();

        // Handle special BLS values
        match trimmed {
            "" | "-" | "N/A" | "n/a" | "NA" => {
                return Err(crate::error::Error::Data(DataError::missing_value(
                    "Empty or missing BLS value".to_string(),
                )));
            }
            _ => {}
        }

        // Validate numeric format
        if !self.numeric_pattern.is_match(trimmed) {
            return Err(crate::error::Error::Data(DataError::invalid_format(
                format!("Invalid numeric format: {value_str}"),
            )));
        }

        trimmed.parse::<f64>().map_err(|e| {
            crate::error::Error::Data(DataError::parse_error(format!(
                "Failed to parse BLS value '{value_str}': {e}"
            )))
        })
    }

    /// Format a BLS value with appropriate precision
    pub fn format_bls_value(&self, value: f64, precision: Option<usize>) -> Result<String> {
        if value.is_nan() || value.is_infinite() {
            return Err(crate::error::Error::Data(DataError::invalid_value(
                format!("Invalid numeric value: {value}"),
            )));
        }

        let precision = precision.unwrap_or(3);
        Ok(format!("{value:.precision$}"))
    }

    /// Sanitize a string by removing invalid characters
    pub fn sanitize_string(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_ascii() && !c.is_control())
            .collect::<String>()
            .trim()
            .to_string()
    }

    /// Normalize whitespace in a string
    pub fn normalize_whitespace(&self, input: &str) -> String {
        input.split_whitespace().collect::<Vec<&str>>().join(" ")
    }

    /// Format a series ID according to BLS standards
    pub fn format_series_id(&self, series_id: &str) -> Result<String> {
        let trimmed = series_id.trim().to_uppercase();

        // BLS series IDs are typically 17-20 characters
        if trimmed.len() < 10 || trimmed.len() > 25 {
            return Err(crate::error::Error::Data(DataError::invalid_format(
                format!("Invalid series ID length: {series_id}"),
            )));
        }

        // Check for valid characters (alphanumeric)
        if !trimmed.chars().all(|c| c.is_alphanumeric()) {
            return Err(crate::error::Error::Data(DataError::invalid_format(
                format!("Invalid characters in series ID: {series_id}"),
            )));
        }

        Ok(trimmed)
    }

    /// Parse a period string (e.g., "M01", "Q01", "A01")
    pub fn parse_period(&self, period_str: &str) -> Result<(char, u32)> {
        let trimmed = period_str.trim().to_uppercase();

        if trimmed.len() != 3 {
            return Err(crate::error::Error::Data(DataError::invalid_format(
                format!("Invalid period format: {period_str}"),
            )));
        }

        let period_type = trimmed.chars().next().unwrap();
        let period_num_str = &trimmed[1..];

        let period_num = period_num_str.parse::<u32>().map_err(|e| {
            crate::error::Error::Data(DataError::parse_error(format!(
                "Failed to parse period number '{period_num_str}': {e}"
            )))
        })?;

        // Validate period type and number
        match period_type {
            'M' => {
                if !(1..=12).contains(&period_num) {
                    return Err(crate::error::Error::Data(DataError::invalid_value(
                        format!("Invalid month period: {period_num}"),
                    )));
                }
            }
            'Q' => {
                if !(1..=4).contains(&period_num) {
                    return Err(crate::error::Error::Data(DataError::invalid_value(
                        format!("Invalid quarter period: {period_num}"),
                    )));
                }
            }
            'A' => {
                if period_num != 1 {
                    return Err(crate::error::Error::Data(DataError::invalid_value(
                        format!("Invalid annual period: {period_num}"),
                    )));
                }
            }
            _ => {
                return Err(crate::error::Error::Data(DataError::invalid_format(
                    format!("Invalid period type: {period_type}"),
                )));
            }
        }

        Ok((period_type, period_num))
    }

    /// Format a period from type and number
    pub fn format_period(&self, period_type: char, period_num: u32) -> Result<String> {
        // Validate inputs first
        self.parse_period(&format!("{period_type}{period_num:02}"))?;
        Ok(format!("{period_type}{period_num:02}"))
    }

    /// Format a number as currency
    pub fn format_currency(&self, value: f64) -> String {
        if value < 0.0 {
            format!("-${:.2}", value.abs())
        } else {
            format!("${value:.2}")
        }
    }

    /// Format a number with commas as thousands separators
    pub fn format_with_commas(&self, value: i64) -> String {
        let result = value.to_string();
        let mut chars: Vec<char> = result.chars().collect();

        // Handle negative numbers
        let start_idx = if chars[0] == '-' { 1 } else { 0 };

        // Add commas from right to left
        let mut i = chars.len();
        while i > start_idx + 3 {
            i -= 3;
            chars.insert(i, ',');
        }

        chars.into_iter().collect()
    }

    /// Format a percentage value
    pub fn format_percentage(&self, value: f64) -> String {
        format!("{:.2}%", value * 100.0)
    }

    /// Truncate a string to a maximum length, adding ellipsis if needed
    pub fn truncate(&self, input: &str, max_len: usize) -> String {
        if input.len() <= max_len {
            input.to_string()
        } else if max_len <= 3 {
            input.chars().take(max_len).collect()
        } else {
            let truncated: String = input.chars().take(max_len - 3).collect();
            format!("{truncated}...")
        }
    }
}

impl Default for FormatUtils {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a BLS value from string
pub fn parse_bls_value(value_str: &str) -> Result<f64> {
    let formatter = FormatUtils::new();
    formatter.parse_bls_value(value_str)
}

/// Format a BLS value to string
pub fn format_bls_value(value: f64) -> Result<String> {
    let formatter = FormatUtils::new();
    formatter.format_bls_value(value, None)
}

/// Format a survey code
pub fn format_survey_code(code: &str) -> Result<String> {
    let formatter = FormatUtils::new();
    formatter.format_survey_code(code)
}

/// Sanitize a string
pub fn sanitize_string(input: &str) -> String {
    let formatter = FormatUtils::new();
    formatter.sanitize_string(input)
}

/// Normalize whitespace in a string
pub fn normalize_whitespace(input: &str) -> String {
    let formatter = FormatUtils::new();
    formatter.normalize_whitespace(input)
}

/// Format a number as currency
pub fn format_currency(value: f64) -> String {
    let formatter = FormatUtils::new();
    formatter.format_currency(value)
}

/// Parse a date string into a standardized format
pub fn parse_date(date_str: &str) -> Result<String> {
    use chrono::NaiveDate;

    let trimmed = date_str.trim();

    // Try different date formats commonly used in BLS data
    let formats = [
        "%Y-%m-%d",  // 2023-01-15
        "%m/%d/%Y",  // 01/15/2023
        "%m-%d-%Y",  // 01-15-2023
        "%Y%m%d",    // 20230115
        "%B %d, %Y", // January 15, 2023
        "%b %d, %Y", // Jan 15, 2023
        "%d/%m/%Y",  // 15/01/2023 (European format)
    ];

    for format in &formats {
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, format) {
            return Ok(date.format("%Y-%m-%d").to_string());
        }
    }

    // If no format matches, return an error
    Err(crate::error::Error::Data(DataError::parse_error(format!(
        "Unable to parse date: {date_str}"
    ))))
}

/// Normalize text by cleaning and standardizing it
pub fn normalize_text(input: &str) -> Result<String> {
    let formatter = FormatUtils::new();

    // First sanitize the string to remove invalid characters
    let sanitized = formatter.sanitize_string(input);

    // Then normalize whitespace
    let normalized = formatter.normalize_whitespace(&sanitized);

    // Convert to title case for consistency
    let title_cased = normalized
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect::<Vec<String>>()
        .join(" ");

    Ok(title_cased)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_utils_new() {
        let formatter = FormatUtils::new();
        // Just ensure it doesn't panic
        assert!(true);
    }

    #[test]
    fn test_format_survey_code() {
        let formatter = FormatUtils::new();

        // Valid codes
        assert_eq!(formatter.format_survey_code("ap").unwrap(), "AP");
        assert_eq!(formatter.format_survey_code("AP").unwrap(), "AP");
        assert_eq!(formatter.format_survey_code(" bd ").unwrap(), "BD");

        // Invalid codes
        assert!(formatter.format_survey_code("a").is_err());
        assert!(formatter.format_survey_code("abc").is_err());
        assert!(formatter.format_survey_code("a1").is_err());
    }

    #[test]
    fn test_parse_bls_value() {
        let formatter = FormatUtils::new();

        // Valid values
        assert_eq!(formatter.parse_bls_value("123.456").unwrap(), 123.456);
        assert_eq!(formatter.parse_bls_value("0").unwrap(), 0.0);
        assert_eq!(formatter.parse_bls_value("-123.456").unwrap(), -123.456);
        assert_eq!(formatter.parse_bls_value("  123.456  ").unwrap(), 123.456);

        // Invalid values
        assert!(formatter.parse_bls_value("").is_err());
        assert!(formatter.parse_bls_value("-").is_err());
        assert!(formatter.parse_bls_value("N/A").is_err());
        assert!(formatter.parse_bls_value("abc").is_err());
    }

    #[test]
    fn test_format_bls_value() {
        let formatter = FormatUtils::new();

        // Valid values
        assert_eq!(
            formatter.format_bls_value(123.456, None).unwrap(),
            "123.456"
        );
        assert_eq!(
            formatter.format_bls_value(123.456, Some(2)).unwrap(),
            "123.46"
        );
        assert_eq!(formatter.format_bls_value(0.0, None).unwrap(), "0.000");

        // Invalid values
        assert!(formatter.format_bls_value(f64::NAN, None).is_err());
        assert!(formatter.format_bls_value(f64::INFINITY, None).is_err());
    }

    #[test]
    fn test_sanitize_string() {
        let formatter = FormatUtils::new();

        assert_eq!(formatter.sanitize_string("Hello, World!"), "Hello, World!");
        assert_eq!(formatter.sanitize_string("  Hello  "), "Hello");
        assert_eq!(formatter.sanitize_string("Hello\nWorld"), "HelloWorld");
        assert_eq!(formatter.sanitize_string("Hello\tWorld"), "HelloWorld");
    }

    #[test]
    fn test_normalize_whitespace() {
        let formatter = FormatUtils::new();

        assert_eq!(
            formatter.normalize_whitespace("Hello    World"),
            "Hello World"
        );
        assert_eq!(
            formatter.normalize_whitespace("  Hello  World  "),
            "Hello World"
        );
        assert_eq!(
            formatter.normalize_whitespace("Hello\n\tWorld"),
            "Hello World"
        );
    }

    #[test]
    fn test_format_series_id() {
        let formatter = FormatUtils::new();

        // Valid series IDs
        assert_eq!(
            formatter.format_series_id("APUS49074714").unwrap(),
            "APUS49074714"
        );
        assert_eq!(
            formatter.format_series_id("apus49074714").unwrap(),
            "APUS49074714"
        );

        // Invalid series IDs
        assert!(formatter.format_series_id("AP").is_err()); // Too short
        assert!(formatter.format_series_id("APUS49074714-INVALID").is_err()); // Too long
        assert!(formatter.format_series_id("APUS490747@14").is_err()); // Invalid characters
    }

    #[test]
    fn test_parse_period() {
        let formatter = FormatUtils::new();

        // Valid periods
        assert_eq!(formatter.parse_period("M01").unwrap(), ('M', 1));
        assert_eq!(formatter.parse_period("M12").unwrap(), ('M', 12));
        assert_eq!(formatter.parse_period("Q01").unwrap(), ('Q', 1));
        assert_eq!(formatter.parse_period("Q04").unwrap(), ('Q', 4));
        assert_eq!(formatter.parse_period("A01").unwrap(), ('A', 1));

        // Invalid periods
        assert!(formatter.parse_period("M00").is_err()); // Invalid month
        assert!(formatter.parse_period("M13").is_err()); // Invalid month
        assert!(formatter.parse_period("Q00").is_err()); // Invalid quarter
        assert!(formatter.parse_period("Q05").is_err()); // Invalid quarter
        assert!(formatter.parse_period("A02").is_err()); // Invalid annual
        assert!(formatter.parse_period("X01").is_err()); // Invalid type
        assert!(formatter.parse_period("M1").is_err()); // Wrong format
    }

    #[test]
    fn test_format_period() {
        let formatter = FormatUtils::new();

        // Valid periods
        assert_eq!(formatter.format_period('M', 1).unwrap(), "M01");
        assert_eq!(formatter.format_period('Q', 4).unwrap(), "Q04");
        assert_eq!(formatter.format_period('A', 1).unwrap(), "A01");

        // Invalid periods
        assert!(formatter.format_period('M', 0).is_err());
        assert!(formatter.format_period('M', 13).is_err());
        assert!(formatter.format_period('X', 1).is_err());
    }

    #[test]
    fn test_convenience_functions() {
        // Test the convenience functions
        assert_eq!(parse_bls_value("123.456").unwrap(), 123.456);
        assert_eq!(format_bls_value(123.456).unwrap(), "123.456");
        assert_eq!(format_survey_code("ap").unwrap(), "AP");
        assert_eq!(sanitize_string("  Hello  "), "Hello");
        assert_eq!(normalize_whitespace("Hello    World"), "Hello World");
    }

    #[test]
    fn test_format_currency() {
        // Test the format_currency function
        assert_eq!(format_currency(123.45), "$123.45");
        assert_eq!(format_currency(0.0), "$0.00");
        assert_eq!(format_currency(-50.25), "-$50.25");
        assert_eq!(format_currency(1234567.89), "$1234567.89");
    }

    #[test]
    fn test_parse_date() {
        // Test the parse_date function
        assert_eq!(parse_date("2023-01-15").unwrap(), "2023-01-15");
        assert_eq!(parse_date("01/15/2023").unwrap(), "2023-01-15");
        assert_eq!(parse_date("20230115").unwrap(), "2023-01-15");

        // Test invalid date
        assert!(parse_date("invalid-date").is_err());
        assert!(parse_date("").is_err());
    }

    #[test]
    fn test_normalize_text() {
        // Test the normalize_text function
        assert_eq!(normalize_text("hello world").unwrap(), "Hello World");
        assert_eq!(normalize_text("  HELLO    WORLD  ").unwrap(), "Hello World");
        assert_eq!(
            normalize_text("hello\nworld\ttab").unwrap(),
            "Hello World Tab"
        );
        assert_eq!(normalize_text("").unwrap(), "");
    }
}
