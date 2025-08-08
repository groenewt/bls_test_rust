//! # Data Module
//!
//! This module provides data structures and operations for the Rusty BLS Data Processing system.
//! It handles BLS data models, reading, and writing operations.
//!
//! ## Components
//!
//! - [`model`]: Data models for BLS data structures (series, observations, lookups, surveys)
//! - [`reader`]: Data readers for various input formats and sources
//! - [`writer`]: Data writers for various output formats and destinations
//!
//! ## Usage
//!
//! ```rust
//! use rusty::data::{Series, Observation, Survey};
//!
//! // Create a new series
//! let series = Series::new("APUS49074714", "Average Price Data");
//!
//! // Create an observation
//! let observation = Observation::new("APUS49074714", &2023, "M01", Some(123.45));
//!
//! // Create a survey
//! let survey = Survey::new("AP", "Average Price Data");
//! ```

// Module declarations
pub mod model;
pub mod reader;
pub mod writer;

// Re-export commonly used types and traits
pub use model::{
    Series, Observation, Lookup, Survey,
    SeriesMetadata, ObservationValue, LookupEntry, SurveyMetadata
};
pub use reader::{
    DataReader, FileReader, MmapReader, ReaderFactory
};
pub use writer::{
    DataWriter, CsvDataWriter, ParquetDataWriter, JsonDataWriter, WriterFactory
};

// Convenience type aliases
pub type Result<T> = std::result::Result<T, crate::error::DataError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all modules are accessible
        // This is a compile-time test to ensure all modules are properly exported
        let _series = Series::new("TEST123", "Test Series");
        let _observation = Observation::new("TEST123", &2023, "M01", Some(100.0));
        let _survey = Survey::new("TS", "Test Survey");
    }
}