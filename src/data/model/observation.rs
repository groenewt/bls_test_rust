//! # Observation Data Model
//!
//! This module defines the Observation data structure for BLS data observations.
//! An observation represents a single data point in a time series.
//!
//! ## Usage
//!

pub use crate::data::model::{CommonMetadata, DataQuality};
use crate::utils::validation::BLSValidationRules;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// BLS data observation
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Observation {
    /// Series identifier
    #[validate(length(min = 10, max = 25))]
    #[validate(custom = "validate_series_id")]
    pub series_id: String,

    /// Year of the observation
    #[validate(range(min = 1900, max = 2100))]
    #[validate(custom = "validate_year")]
    pub year: u32,

    /// Period within the year (e.g., "M01", "Q01", "A01")
    #[validate(length(min = 3, max = 3))]
    #[validate(custom = "validate_period")]
    pub period: String,

    /// The observation value
    pub value: ObservationValue,

    /// Common metadata (timestamps, version, etc.)
    #[validate]
    pub common: CommonMetadata,
}

impl Observation {
    /// Create a new observation
    pub fn new(series_id: &str, year: &i32, period: &str, value: Option<f64>) -> Self {
        Self {
            series_id: series_id.to_string(),
            year: *year as u32,
            period: period.to_string(),
            value: ObservationValue::new(value),
            common: CommonMetadata::new(),
        }
    }

    /// Create an observation builder
    pub fn builder() -> ObservationBuilder {
        ObservationBuilder::new()
    }

    /// Get the series ID
    pub fn series_id(&self) -> &str {
        &self.series_id
    }

    /// Get the year
    pub fn year(&self) -> u32 {
        self.year
    }

    /// Get the period
    pub fn period(&self) -> &str {
        &self.period
    }

    /// Get the numeric value if present
    pub fn numeric_value(&self) -> Option<f64> {
        self.value.numeric_value()
    }

    /// Check if the observation has a value
    pub fn has_value(&self) -> bool {
        self.value.has_value()
    }

    /// Check if the observation is missing
    pub fn is_missing(&self) -> bool {
        !self.has_value()
    }

    /// Get the data quality
    pub fn quality(&self) -> &DataQuality {
        &self.value.quality
    }

    /// Set the value
    pub fn set_value(&mut self, value: Option<f64>) {
        self.value.set_value(value);
        self.common.update();
    }

    /// Set the data quality
    pub fn set_quality(&mut self, quality: DataQuality) {
        self.value.quality = quality;
        self.common.update();
    }

    /// Add a footnote
    pub fn add_footnote(&mut self, footnote: &str) {
        self.value.add_footnote(footnote);
        self.common.update();
    }

    /// Add a custom attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.common.add_attribute(key, value);
    }

    /// Create a unique key for this observation
    pub fn key(&self) -> String {
        format!("{}:{}:{}", self.series_id, self.year, self.period)
    }

    /// Parse period into type and number
    pub fn parse_period(&self) -> Result<(char, u32), String> {
        crate::utils::format::FormatUtils::new()
            .parse_period(&self.period)
            .map_err(|e| e.to_string())
    }

    /// Get the period type (M, Q, A, etc.)
    pub fn period_type(&self) -> Option<char> {
        self.parse_period().ok().map(|(period_type, _)| period_type)
    }

    /// Get the period number
    pub fn period_number(&self) -> Option<u32> {
        self.parse_period().ok().map(|(_, period_num)| period_num)
    }

    /// Validate the observation
    pub fn validate_observation(&self) -> Result<(), String> {
        // Validate using BLS rules
        BLSValidationRules::validate_observation(
            &self.series_id,
            self.year,
            &self.period,
            self.numeric_value(),
        )
        .map_err(|e| e.to_string())?;

        // Validate using validator crate
        self.validate()
            .map_err(|e| format!("Validation error: {e:?}"))?;

        Ok(())
    }

    /// Compare observations for sorting
    pub fn compare_time(&self, other: &Self) -> std::cmp::Ordering {
        match self.year.cmp(&other.year) {
            std::cmp::Ordering::Equal => self.period.cmp(&other.period),
            other => other,
        }
    }
}

/// Observation value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationValue {
    /// The numeric value (None for missing values)
    pub value: Option<f64>,

    /// Data quality indicator
    pub quality: DataQuality,

    /// Footnotes or annotations
    #[serde(default)]
    pub footnotes: Vec<String>,

    /// Value status (e.g., "preliminary", "revised")
    pub status: Option<String>,

    /// Custom attributes for this value
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl ObservationValue {
    /// Create a new observation value
    pub fn new(value: Option<f64>) -> Self {
        Self {
            value,
            quality: DataQuality::High,
            footnotes: Vec::new(),
            status: None,
            attributes: HashMap::new(),
        }
    }

    /// Create an observation value with quality
    pub fn with_quality(value: Option<f64>, quality: DataQuality) -> Self {
        Self {
            value,
            quality,
            footnotes: Vec::new(),
            status: None,
            attributes: HashMap::new(),
        }
    }

    /// Get the numeric value
    pub fn numeric_value(&self) -> Option<f64> {
        self.value
    }

    /// Check if the value is present
    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    /// Set the value
    pub fn set_value(&mut self, value: Option<f64>) {
        self.value = value;
    }

    /// Add a footnote
    pub fn add_footnote(&mut self, footnote: &str) {
        if !self.footnotes.contains(&footnote.to_string()) {
            self.footnotes.push(footnote.to_string());
        }
    }

    /// Remove a footnote
    pub fn remove_footnote(&mut self, footnote: &str) {
        self.footnotes.retain(|f| f != footnote);
    }

    /// Check if value has a specific footnote
    pub fn has_footnote(&self, footnote: &str) -> bool {
        self.footnotes.contains(&footnote.to_string())
    }

    /// Set the status
    pub fn set_status(&mut self, status: &str) {
        self.status = Some(status.to_string());
    }

    /// Add a custom attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }

    /// Check if the value is valid (not NaN or infinite)
    pub fn is_valid(&self) -> bool {
        match self.value {
            Some(val) => !val.is_nan() && !val.is_infinite(),
            None => true, // Missing values are considered valid
        }
    }

    /// Format the value for display
    pub fn format_value(&self, precision: Option<usize>) -> String {
        match self.value {
            Some(val) => {
                let precision = precision.unwrap_or(3);
                format!("{val:.precision$}")
            }
            None => "N/A".to_string(),
        }
    }
}

impl Default for ObservationValue {
    fn default() -> Self {
        Self::new(None)
    }
}

/// Builder for Observation
pub struct ObservationBuilder {
    series_id: Option<String>,
    year: Option<u32>,
    period: Option<String>,
    value: Option<f64>,
    quality: DataQuality,
    footnotes: Vec<String>,
    status: Option<String>,
    attributes: HashMap<String, String>,
}

impl ObservationBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            series_id: None,
            year: None,
            period: None,
            value: None,
            quality: DataQuality::High,
            footnotes: Vec::new(),
            status: None,
            attributes: HashMap::new(),
        }
    }

    /// Set the series ID
    pub fn series_id(mut self, series_id: &str) -> Self {
        self.series_id = Some(series_id.to_string());
        self
    }

    /// Set the year
    pub fn year(mut self, year: u32) -> Self {
        self.year = Some(year);
        self
    }

    /// Set the period
    pub fn period(mut self, period: &str) -> Self {
        self.period = Some(period.to_string());
        self
    }

    /// Set the value
    pub fn value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self
    }

    /// Set missing value
    pub fn missing_value(mut self) -> Self {
        self.value = None;
        self
    }

    /// Set the quality
    pub fn quality(mut self, quality: DataQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Add a footnote
    pub fn footnote(mut self, footnote: &str) -> Self {
        if !self.footnotes.contains(&footnote.to_string()) {
            self.footnotes.push(footnote.to_string());
        }
        self
    }

    /// Set the status
    pub fn status(mut self, status: &str) -> Self {
        self.status = Some(status.to_string());
        self
    }

    /// Add an attribute
    pub fn attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    /// Build the observation
    pub fn build(self) -> Result<Observation, String> {
        let series_id = self.series_id.ok_or("Series ID is required")?;
        let year = self.year.ok_or("Year is required")?;
        let period = self.period.ok_or("Period is required")?;

        let mut obs_value = ObservationValue::new(self.value);
        obs_value.quality = self.quality;
        obs_value.footnotes = self.footnotes;
        obs_value.status = self.status;
        obs_value.attributes = self.attributes;

        let observation = Observation {
            series_id,
            year,
            period,
            value: obs_value,
            common: CommonMetadata::new(),
        };

        // Validate the observation
        observation.validate_observation()?;

        Ok(observation)
    }
}

impl Default for ObservationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Custom validation functions
fn validate_series_id(series_id: &str) -> Result<(), validator::ValidationError> {
    crate::utils::validation::validate_series_id(series_id).map_err(|e| {
        let mut err = validator::ValidationError::new("invalid_series_id");
        let error_msg = e.to_string();
        err.params
            .insert("error".into(), serde_json::Value::String(error_msg));
        err
    })
}

fn validate_year(year: u32) -> Result<(), validator::ValidationError> {
    crate::utils::validation::validate_year(year).map_err(|e| {
        let mut err = validator::ValidationError::new("invalid_year");
        let error_msg = e.to_string();
        err.params
            .insert("error".into(), serde_json::Value::String(error_msg));
        err
    })
}

fn validate_period(period: &str) -> Result<(), validator::ValidationError> {
    crate::utils::validation::validate_period(period).map_err(|e| {
        let mut err = validator::ValidationError::new("invalid_period");
        let error_msg = e.to_string();
        err.params
            .insert("error".into(), serde_json::Value::String(error_msg));
        err
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observation_new() {
        let obs = Observation::new("APUS49074714", &2023, "M01", Some(3.45));
        assert_eq!(obs.series_id, "APUS49074714");
        assert_eq!(obs.year, 2023);
        assert_eq!(obs.period, "M01");
        assert_eq!(obs.numeric_value(), Some(3.45));
        assert!(obs.has_value());
        assert!(!obs.is_missing());
    }

    #[test]
    fn test_observation_missing_value() {
        let obs = Observation::new("APUS49074714", &2023, "M02", None);
        assert_eq!(obs.numeric_value(), None);
        assert!(!obs.has_value());
        assert!(obs.is_missing());
    }

    #[test]
    fn test_observation_key() {
        let obs = Observation::new("APUS49074714", &2023, "M01", Some(3.45));
        assert_eq!(obs.key(), "APUS49074714:2023:M01");
    }

    #[test]
    fn test_parse_period() {
        let obs = Observation::new("APUS49074714", &2023, "M01", Some(3.45));
        let (period_type, period_num) = obs.parse_period().unwrap();
        assert_eq!(period_type, 'M');
        assert_eq!(period_num, 1);
        assert_eq!(obs.period_type(), Some('M'));
        assert_eq!(obs.period_number(), Some(1));
    }

    #[test]
    fn test_observation_builder() {
        let obs = Observation::builder()
            .series_id("APUS49074714")
            .year(2023)
            .period("M01")
            .value(3.45)
            .quality(DataQuality::Preliminary)
            .footnote("Test footnote")
            .status("preliminary")
            .attribute("source", "BLS")
            .build()
            .unwrap();

        assert_eq!(obs.series_id, "APUS49074714");
        assert_eq!(obs.year, 2023);
        assert_eq!(obs.period, "M01");
        assert_eq!(obs.numeric_value(), Some(3.45));
        assert_eq!(obs.quality(), &DataQuality::Preliminary);
        assert!(obs.value.has_footnote("Test footnote"));
        assert_eq!(obs.value.status, Some("preliminary".to_string()));
        assert_eq!(obs.value.attributes.get("source"), Some(&"BLS".to_string()));
    }

    #[test]
    fn test_observation_builder_missing_fields() {
        let result = Observation::builder()
            .series_id("APUS49074714")
            .year(2023)
            // Missing period
            .value(3.45)
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Period is required"));
    }

    #[test]
    fn test_observation_value() {
        let mut obs_value = ObservationValue::new(Some(123.45));
        assert_eq!(obs_value.numeric_value(), Some(123.45));
        assert!(obs_value.has_value());
        assert!(obs_value.is_valid());

        obs_value.add_footnote("Test footnote");
        assert!(obs_value.has_footnote("Test footnote"));
        assert_eq!(obs_value.footnotes.len(), 1);

        obs_value.add_footnote("Test footnote"); // Duplicate should not be added
        assert_eq!(obs_value.footnotes.len(), 1);

        obs_value.remove_footnote("Test footnote");
        assert!(!obs_value.has_footnote("Test footnote"));
        assert_eq!(obs_value.footnotes.len(), 0);
    }

    #[test]
    fn test_observation_value_format() {
        let obs_value = ObservationValue::new(Some(123.456));
        assert_eq!(obs_value.format_value(None), "123.456");
        assert_eq!(obs_value.format_value(Some(2)), "123.46");

        let missing_value = ObservationValue::new(None);
        assert_eq!(missing_value.format_value(None), "N/A");
    }

    #[test]
    fn test_observation_value_validity() {
        let valid_value = ObservationValue::new(Some(123.45));
        assert!(valid_value.is_valid());

        let nan_value = ObservationValue::new(Some(f64::NAN));
        assert!(!nan_value.is_valid());

        let infinite_value = ObservationValue::new(Some(f64::INFINITY));
        assert!(!infinite_value.is_valid());

        let missing_value = ObservationValue::new(None);
        assert!(missing_value.is_valid()); // Missing values are valid
    }

    #[test]
    fn test_observation_comparison() {
        let obs1 = Observation::new("APUS49074714", &2023, "M01", Some(3.45));
        let obs2 = Observation::new("APUS49074714", &2023, "M02", Some(3.50));
        let obs3 = Observation::new("APUS49074714", &2024, "M01", Some(3.55));

        assert_eq!(obs1.compare_time(&obs2), std::cmp::Ordering::Less);
        assert_eq!(obs1.compare_time(&obs3), std::cmp::Ordering::Less);
        assert_eq!(obs2.compare_time(&obs3), std::cmp::Ordering::Less);
        assert_eq!(obs1.compare_time(&obs1), std::cmp::Ordering::Equal);
    }

    #[test]
    fn test_observation_updates() {
        let mut obs = Observation::new("APUS49074714", &2023, "M01", Some(3.45));
        let original_version = obs.common.version;

        obs.set_value(Some(3.50));
        assert_eq!(obs.numeric_value(), Some(3.50));
        assert!(obs.common.version > original_version);

        let new_version = obs.common.version;
        obs.set_quality(DataQuality::Preliminary);
        assert_eq!(obs.quality(), &DataQuality::Preliminary);
        assert!(obs.common.version > new_version);
    }

    #[test]
    fn test_observation_validation() {
        let valid_obs = Observation::new("APUS49074714", &2023, "M01", Some(3.45));
        assert!(valid_obs.validate_observation().is_ok());

        let invalid_obs = Observation::new("X", &2023, "M01", Some(3.45)); // Invalid series ID
        assert!(invalid_obs.validate_observation().is_err());
    }

    #[test]
    fn test_serialization() {
        let obs = Observation::new("APUS49074714", &2023, "M01", Some(3.45));
        let serialized = serde_json::to_string(&obs).unwrap();
        let deserialized: Observation = serde_json::from_str(&serialized).unwrap();

        assert_eq!(obs.series_id, deserialized.series_id);
        assert_eq!(obs.year, deserialized.year);
        assert_eq!(obs.period, deserialized.period);
        assert_eq!(obs.numeric_value(), deserialized.numeric_value());
    }
}
