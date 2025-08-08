//! # Series Data Model
//!
//! This module defines the Series data structure for BLS data series.
//! A series represents a collection of related data observations over time.
//!
//! ## Usage
//!

use crate::data::model::{Area, CommonMetadata, DataQuality, DataStatus, Frequency, Item, Unit};
use crate::utils::validation::BLSValidationRules;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

/// BLS data series
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Series {
    /// Series identifier (e.g., "APUS49074714")
    #[validate(length(min = 10, max = 25))]
    #[validate(custom = "validate_series_id")]
    pub series_id: String,

    /// Series title/description
    #[validate(length(min = 1, max = 500))]
    pub title: String,

    /// Survey code (e.g., "AP")
    #[validate(length(min = 2, max = 2))]
    #[validate(custom = "validate_survey_code")]
    pub survey_code: String,

    /// Series metadata
    #[validate]
    pub metadata: SeriesMetadata,

    /// Common metadata (timestamps, version, etc.)
    #[validate]
    pub common: CommonMetadata,

    /// Periodicity code
    pub periodicity_code: String,

    /// Seasonal adjustment code
    pub seasonal: String,

    /// Item code
    pub item_code: String,

    /// Base period
    pub base_period: String,

    /// Base code
    pub base_code: String,
    pub area_code: String,
}

impl Series {
    /// Create a new series with minimal information
    pub fn new(series_id: &str, title: &str) -> Self {
        let survey_code = extract_survey_code(series_id);
        Self {
            series_id: series_id.to_string(),
            title: title.to_string(),
            survey_code,
            metadata: SeriesMetadata::default(),
            common: CommonMetadata::new(),
            periodicity_code: String::new(),
            seasonal: String::new(),
            item_code: String::new(),
            base_period: String::new(),
            base_code: String::new(),
            area_code: String::new(),
        }
    }

    /// Create a series with full metadata
    pub fn with_metadata(
        series_id: &str,
        title: &str,
        area_code: &str,
        metadata: SeriesMetadata,
    ) -> Self {
        let survey_code = extract_survey_code(series_id);
        Self {
            series_id: series_id.to_string(),
            title: title.to_string(),
            survey_code,
            area_code: area_code.to_string(),
            metadata,
            common: CommonMetadata::new(),
            periodicity_code: String::new(),
            seasonal: String::new(),
            item_code: String::new(),
            base_period: String::new(),
            base_code: String::new(),
        }
    }

    /// Get the series ID
    pub fn id(&self) -> &str {
        &self.series_id
    }

    /// Get the series title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the survey code
    pub fn survey_code(&self) -> &str {
        &self.survey_code
    }

    /// Check if the series is active
    pub fn is_active(&self) -> bool {
        self.metadata.status == DataStatus::Active
    }

    /// Get the data frequency
    pub fn frequency(&self) -> &Frequency {
        &self.metadata.frequency
    }

    /// Get the unit of measurement
    pub fn unit(&self) -> Option<&Unit> {
        self.metadata.unit.as_ref()
    }

    /// Get the geographic area
    pub fn area(&self) -> Option<&Area> {
        self.metadata.area.as_ref()
    }

    /// Get the item/industry
    pub fn item(&self) -> Option<&Item> {
        self.metadata.item.as_ref()
    }

    /// Update the series metadata
    pub fn update_metadata(&mut self, metadata: SeriesMetadata) {
        self.metadata = metadata;
        self.common.update();
    }

    /// Set the series status
    pub fn set_status(&mut self, status: DataStatus) {
        self.metadata.status = status;
        self.common.update();
    }

    /// Add a custom attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.common.add_attribute(key, value);
    }

    /// Validate the series
    pub fn validate_series(&self) -> Result<(), String> {
        // Validate using BLS rules
        BLSValidationRules::validate_series_metadata(
            &self.series_id,
            &self.title,
            &self.survey_code,
        )
        .map_err(|e| e.to_string())?;

        // Validate using validator crate
        self.validate()
            .map_err(|e| format!("Validation error: {e:?}"))?;

        Ok(())
    }
}

/// Series metadata containing detailed information about the series
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SeriesMetadata {
    /// Data frequency
    pub frequency: Frequency,

    /// Data status
    pub status: DataStatus,

    /// Data quality indicator
    pub quality: DataQuality,

    /// Unit of measurement
    pub unit: Option<Unit>,

    /// Geographic area
    pub area: Option<Area>,

    /// Item/industry information
    pub item: Option<Item>,

    /// Start date of the series
    pub start_date: Option<DateTime<Utc>>,

    /// End date of the series (if discontinued)
    pub end_date: Option<DateTime<Utc>>,

    /// Last update date
    pub last_updated: Option<DateTime<Utc>>,

    /// Seasonal adjustment indicator
    pub seasonal_adjustment: Option<String>,

    /// Base period for index series
    pub base_period: Option<String>,

    /// Additional notes or comments
    pub notes: Option<String>,

    /// Custom attributes
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl SeriesMetadata {
    /// Create a new metadata builder
    pub fn builder() -> SeriesMetadataBuilder {
        SeriesMetadataBuilder::new()
    }

    /// Check if the series has seasonal adjustment
    pub fn is_seasonally_adjusted(&self) -> bool {
        self.seasonal_adjustment
            .as_ref()
            .map(|sa| sa.to_lowercase().contains("seasonally adjusted"))
            .unwrap_or(false)
    }

    /// Get the effective end date (end_date or current time if active)
    pub fn effective_end_date(&self) -> DateTime<Utc> {
        self.end_date.unwrap_or_else(Utc::now)
    }

    /// Add a custom attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.attributes.insert(key.to_string(), value.to_string());
    }
}

impl Default for SeriesMetadata {
    fn default() -> Self {
        Self {
            frequency: Frequency::Monthly,
            status: DataStatus::Active,
            quality: DataQuality::High,
            unit: None,
            area: None,
            item: None,
            start_date: None,
            end_date: None,
            last_updated: None,
            seasonal_adjustment: None,
            base_period: None,
            notes: None,
            attributes: HashMap::new(),
        }
    }
}

/// Builder for SeriesMetadata
pub struct SeriesMetadataBuilder {
    metadata: SeriesMetadata,
}

impl SeriesMetadataBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            metadata: SeriesMetadata::default(),
        }
    }

    /// Set the frequency
    pub fn frequency(mut self, frequency: Frequency) -> Self {
        self.metadata.frequency = frequency;
        self
    }

    /// Set the status
    pub fn status(mut self, status: DataStatus) -> Self {
        self.metadata.status = status;
        self
    }

    /// Set the quality
    pub fn quality(mut self, quality: DataQuality) -> Self {
        self.metadata.quality = quality;
        self
    }

    /// Set the unit
    pub fn unit(mut self, unit: Unit) -> Self {
        self.metadata.unit = Some(unit);
        self
    }

    /// Set the area
    pub fn area(mut self, area: Area) -> Self {
        self.metadata.area = Some(area);
        self
    }

    /// Set the item
    pub fn item(mut self, item: Item) -> Self {
        self.metadata.item = Some(item);
        self
    }

    /// Set the start date
    pub fn start_date(mut self, start_date: DateTime<Utc>) -> Self {
        self.metadata.start_date = Some(start_date);
        self
    }

    /// Set the end date
    pub fn end_date(mut self, end_date: DateTime<Utc>) -> Self {
        self.metadata.end_date = Some(end_date);
        self
    }

    /// Set the last updated date
    pub fn last_updated(mut self, last_updated: DateTime<Utc>) -> Self {
        self.metadata.last_updated = Some(last_updated);
        self
    }

    /// Set seasonal adjustment
    pub fn seasonal_adjustment(mut self, seasonal_adjustment: &str) -> Self {
        self.metadata.seasonal_adjustment = Some(seasonal_adjustment.to_string());
        self
    }

    /// Set base period
    pub fn base_period(mut self, base_period: &str) -> Self {
        self.metadata.base_period = Some(base_period.to_string());
        self
    }

    /// Set notes
    pub fn notes(mut self, notes: &str) -> Self {
        self.metadata.notes = Some(notes.to_string());
        self
    }

    /// Add an attribute
    pub fn attribute(mut self, key: &str, value: &str) -> Self {
        self.metadata.add_attribute(key, value);
        self
    }

    /// Build the metadata
    pub fn build(self) -> SeriesMetadata {
        self.metadata
    }
}

impl Default for SeriesMetadataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract survey code from series ID
fn extract_survey_code(series_id: &str) -> String {
    // BLS series IDs typically start with the survey code
    // e.g., "APUS49074714" -> "AP"
    if series_id.len() >= 2 {
        series_id[0..2].to_uppercase()
    } else {
        "XX".to_string() // Default fallback
    }
}

// Custom validation functions
fn validate_series_id(series_id: &str) -> Result<(), validator::ValidationError> {
    crate::utils::validation::validate_series_id(series_id).map_err(|e| {
        let mut error = validator::ValidationError::new("invalid_series_id");
        error.message = Some(format!("Invalid series ID: {e}").into());
        error
    })
}

fn validate_survey_code(survey_code: &str) -> Result<(), validator::ValidationError> {
    crate::utils::validation::validate_survey_code(survey_code).map_err(|e| {
        let mut error = validator::ValidationError::new("invalid_survey_code");
        error.message = Some(format!("Invalid survey code: {e}").into());
        error
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_series_new() {
        let series = Series::new("APUS49074714", "Average Price Data - Gasoline");
        assert_eq!(series.series_id, "APUS49074714");
        assert_eq!(series.title, "Average Price Data - Gasoline");
        assert_eq!(series.survey_code, "AP");
        assert!(series.is_active());
    }

    #[test]
    fn test_extract_survey_code() {
        assert_eq!(extract_survey_code("APUS49074714"), "AP");
        assert_eq!(extract_survey_code("BDUS00000000"), "BD");
        assert_eq!(extract_survey_code("X"), "XX"); // Fallback for short IDs
    }

    #[test]
    fn test_series_metadata_builder() {
        let unit = Unit::with_symbol("USD", "US Dollars", "$");
        let area = Area::new("US", "United States", "Country");

        let metadata = SeriesMetadata::builder()
            .frequency(Frequency::Monthly)
            .status(DataStatus::Active)
            .quality(DataQuality::High)
            .unit(unit)
            .area(area)
            .seasonal_adjustment("Seasonally Adjusted")
            .notes("Test series")
            .attribute("test_key", "test_value")
            .build();

        assert_eq!(metadata.frequency, Frequency::Monthly);
        assert_eq!(metadata.status, DataStatus::Active);
        assert_eq!(metadata.quality, DataQuality::High);
        assert!(metadata.unit.is_some());
        assert!(metadata.area.is_some());
        assert!(metadata.is_seasonally_adjusted());
        assert_eq!(metadata.notes, Some("Test series".to_string()));
        assert_eq!(
            metadata.attributes.get("test_key"),
            Some(&"test_value".to_string())
        );
    }

    #[test]
    fn test_series_with_metadata() {
        let metadata = SeriesMetadata::builder()
            .frequency(Frequency::Quarterly)
            .status(DataStatus::Inactive)
            .build();

        let series =
            Series::with_metadata("BDUS00000000", "Business Dynamics", "US00000000", metadata);
        assert_eq!(series.frequency(), &Frequency::Quarterly);
        assert!(!series.is_active());
    }

    #[test]
    fn test_series_update_metadata() {
        let mut series = Series::new("APUS49074714", "Test Series");
        let original_version = series.common.version;

        let new_metadata = SeriesMetadata::builder()
            .status(DataStatus::Discontinued)
            .build();

        series.update_metadata(new_metadata);
        assert_eq!(series.metadata.status, DataStatus::Discontinued);
        assert!(series.common.version > original_version);
    }

    #[test]
    fn test_series_attributes() {
        let mut series = Series::new("APUS49074714", "Test Series");
        series.add_attribute("source", "BLS");
        series.add_attribute("category", "Energy");

        assert_eq!(
            series.common.attributes.get("source"),
            Some(&"BLS".to_string())
        );
        assert_eq!(
            series.common.attributes.get("category"),
            Some(&"Energy".to_string())
        );
    }

    #[test]
    fn test_series_validation() {
        let series = Series::new("APUS49074714", "Valid Series");
        assert!(series.validate_series().is_ok());

        let invalid_series = Series::new("X", "Invalid Series"); // Too short series ID
        assert!(invalid_series.validate_series().is_err());
    }

    #[test]
    fn test_seasonal_adjustment() {
        let mut metadata = SeriesMetadata::default();
        assert!(!metadata.is_seasonally_adjusted());

        metadata.seasonal_adjustment = Some("Seasonally Adjusted".to_string());
        assert!(metadata.is_seasonally_adjusted());

        metadata.seasonal_adjustment = Some("Not Seasonally Adjusted".to_string());
        assert!(!metadata.is_seasonally_adjusted());
    }

    #[test]
    fn test_effective_end_date() {
        let metadata = SeriesMetadata::default();
        let end_date = metadata.effective_end_date();
        // Should be approximately current time since no end_date is set
        assert!((Utc::now() - end_date).num_seconds().abs() < 5);

        let mut metadata_with_end = SeriesMetadata::default();
        let specific_end = Utc::now() - chrono::Duration::days(30);
        metadata_with_end.end_date = Some(specific_end);
        assert_eq!(metadata_with_end.effective_end_date(), specific_end);
    }

    #[test]
    fn test_serialization() {
        let series = Series::new("APUS49074714", "Test Series");
        let serialized = serde_json::to_string(&series).unwrap();
        let deserialized: Series = serde_json::from_str(&serialized).unwrap();

        assert_eq!(series.series_id, deserialized.series_id);
        assert_eq!(series.title, deserialized.title);
        assert_eq!(series.survey_code, deserialized.survey_code);
    }
}
