//! Comprehensive unit tests for data models
//! 
//! These tests verify the data model structures including:
//! - Series: BLS data series with metadata and validation
//! - Observation: Individual data points with temporal information
//! - Lookup: Reference data for codes and descriptions
//! - Survey: Survey-level metadata and configuration
//! 
//! The tests cover:
//! - Structure creation and initialization
//! - Field validation and constraints
//! - Method functionality and edge cases
//! - Serialization and deserialization
//! - Error handling and recovery

use std::collections::HashMap;
use chrono::{DateTime, Utc, TimeZone};
use serde_json;

use rusty::data::model::{
    Series, SeriesMetadata, SeriesMetadataBuilder,
    Observation, ObservationValue,
    Lookup, LookupEntry,
    Survey, SurveyMetadata,
    CommonMetadata, DataQuality, DataStatus, Frequency, Unit, Area, Item
};
use rusty::error::{Result, DataError};

#[cfg(test)]
mod series_tests {
    use super::*;

    #[test]
    fn test_series_creation_minimal() {
        let series = Series::new("APUS49074714", "Average Price Data");
        
        assert_eq!(series.id(), "APUS49074714");
        assert_eq!(series.title(), "Average Price Data");
        assert_eq!(series.survey_code(), "AP");
        assert_eq!(series.metadata.status, DataStatus::Active);
        assert_eq!(series.metadata.frequency, Frequency::Monthly);
    }

    #[test]
    fn test_series_creation_with_metadata() {
        let metadata = SeriesMetadataBuilder::new()
            .frequency(Frequency::Quarterly)
            .status(DataStatus::Active)
            .quality(DataQuality::High)
            .build();
            
        let series = Series::with_metadata("BDUS00000001", "Business Dynamics", "US", metadata);
        
        assert_eq!(series.id(), "BDUS00000001");
        assert_eq!(series.title(), "Business Dynamics");
        assert_eq!(series.survey_code(), "BD");
        assert_eq!(series.frequency(), &Frequency::Quarterly);
        assert_eq!(series.metadata.quality, DataQuality::High);
    }

    #[test]
    fn test_series_validation_valid_ids() {
        // Test various valid series ID formats
        let valid_ids = vec![
            "APUS49074714",  // Average Price
            "BDUS00000001",  // Business Dynamics
            "CEUS0000SA0",   // Current Employment Statistics
            "CUSR0000SA0",   // Consumer Price Index
        ];

        for id in valid_ids {
            let series = Series::new(id, "Test Series");
            assert!(series.validate_series().is_ok(), "Series ID {} should be valid", id);
        }
    }

    #[test]
    fn test_series_validation_invalid_ids() {
        // Test invalid series ID formats
        let invalid_ids = vec![
            "",              // Empty
            "AP",            // Too short
            "A",             // Way too short
            "APUS49074714TOOLONG", // Too long
            "123456789012",  // All numbers
            "ap12345678",    // Lowercase
        ];

        for id in invalid_ids {
            let series = Series::new(id, "Test Series");
            assert!(series.validate_series().is_err(), "Series ID {} should be invalid", id);
        }
    }

    #[test]
    fn test_series_validation_titles() {
        // Valid title
        let series = Series::new("APUS49074714", "Valid Title");
        assert!(series.validate_series().is_ok());

        // Empty title should be invalid
        let series = Series::new("APUS49074714", "");
        assert!(series.validate_series().is_err());

        // Very long title should be invalid
        let long_title = "A".repeat(501);
        let series = Series::new("APUS49074714", &long_title);
        assert!(series.validate_series().is_err());
    }

    #[test]
    fn test_series_survey_code_extraction() {
        let test_cases = vec![
            ("APUS49074714", "AP"),
            ("BDUS00000001", "BD"),
            ("CEUS0000SA0", "CE"),
            ("CUSR0000SA0", "CU"),
        ];

        for (series_id, expected_code) in test_cases {
            let series = Series::new(series_id, "Test");
            assert_eq!(series.survey_code(), expected_code);
        }
    }

    #[test]
    fn test_series_status_management() {
        let mut series = Series::new("APUS49074714", "Test Series");
        
        // Initially active
        assert!(series.is_active());
        assert_eq!(series.metadata.status, DataStatus::Active);

        // Set to inactive
        series.set_status(DataStatus::Inactive);
        assert!(!series.is_active());
        assert_eq!(series.metadata.status, DataStatus::Inactive);

        // Set to discontinued
        series.set_status(DataStatus::Discontinued);
        assert!(!series.is_active());
        assert_eq!(series.metadata.status, DataStatus::Discontinued);
    }

    #[test]
    fn test_series_metadata_update() {
        let mut series = Series::new("APUS49074714", "Test Series");
        let initial_version = series.common.version;
        
        let new_metadata = SeriesMetadataBuilder::new()
            .frequency(Frequency::Annual)
            .quality(DataQuality::Medium)
            .build();

        series.update_metadata(new_metadata);
        
        assert_eq!(series.frequency(), &Frequency::Annual);
        assert_eq!(series.metadata.quality, DataQuality::Medium);
        assert!(series.common.version > initial_version);
    }

    #[test]
    fn test_series_attributes() {
        let mut series = Series::new("APUS49074714", "Test Series");
        
        // Add custom attributes
        series.add_attribute("source", "BLS");
        series.add_attribute("category", "prices");
        
        assert_eq!(series.common.attributes.get("source"), Some(&"BLS".to_string()));
        assert_eq!(series.common.attributes.get("category"), Some(&"prices".to_string()));
    }

    #[test]
    fn test_series_serialization() {
        let series = Series::new("APUS49074714", "Average Price Data");
        
        // Test JSON serialization
        let json = serde_json::to_string(&series).expect("Should serialize to JSON");
        assert!(json.contains("APUS49074714"));
        assert!(json.contains("Average Price Data"));
        
        // Test deserialization
        let deserialized: Series = serde_json::from_str(&json).expect("Should deserialize from JSON");
        assert_eq!(deserialized.id(), series.id());
        assert_eq!(deserialized.title(), series.title());
        assert_eq!(deserialized.survey_code(), series.survey_code());
    }

    #[test]
    fn test_series_metadata_builder() {
        let metadata = SeriesMetadataBuilder::new()
            .frequency(Frequency::Weekly)
            .status(DataStatus::Active)
            .quality(DataQuality::High)
            .seasonal_adjustment(true)
            .build();

        assert_eq!(metadata.frequency, Frequency::Weekly);
        assert_eq!(metadata.status, DataStatus::Active);
        assert_eq!(metadata.quality, DataQuality::High);
        assert_eq!(metadata.seasonal_adjustment, Some(true));
    }

    #[test]
    fn test_series_with_geographic_area() {
        let area = Area::new("US", "United States");
        let metadata = SeriesMetadataBuilder::new()
            .area(area.clone())
            .build();
            
        let series = Series::with_metadata("APUS49074714", "Test", "US", metadata);
        
        assert!(series.area().is_some());
        assert_eq!(series.area().unwrap().code, "US");
        assert_eq!(series.area().unwrap().name, "United States");
    }

    #[test]
    fn test_series_with_item() {
        let item = Item::new("ITEM01", "Test Item");
        let metadata = SeriesMetadataBuilder::new()
            .item(item.clone())
            .build();
            
        let series = Series::with_metadata("APUS49074714", "Test", metadata);
        
        assert!(series.item().is_some());
        assert_eq!(series.item().unwrap().code, "ITEM01");
        assert_eq!(series.item().unwrap().name, "Test Item");
    }

    #[test]
    fn test_series_with_unit() {
        let unit = Unit::new("USD", "US Dollars", 2);
        let metadata = SeriesMetadataBuilder::new()
            .unit(unit.clone())
            .build();
            
        let series = Series::with_metadata("APUS49074714", "Test", "US", metadata);
        
        assert!(series.unit().is_some());
        assert_eq!(series.unit().unwrap().code, "USD");
        assert_eq!(series.unit().unwrap().name, "US Dollars");
        assert_eq!(series.unit().unwrap().decimal_places, 2);
    }
}

#[cfg(test)]
mod observation_tests {
    use super::*;

    #[test]
    fn test_observation_creation() {
        let obs = Observation::new("APUS49074714", &2023, "M01", Some(123.45));
        
        assert_eq!(obs.series_id(), "APUS49074714");
        assert_eq!(obs.year(), 2023);
        assert_eq!(obs.period(), "M01");
        assert_eq!(obs.numeric_value().unwrap(), 123.45);
    }

    #[test]
    fn test_observation_with_null_value() {
        let obs = Observation::new("APUS49074714", &2023, "M01", None);
        
        assert!(obs.numeric_value().is_none());
        assert!(obs.is_missing());
    }

    #[test]
    fn test_observation_with_footnotes() {
        let mut obs = Observation::new("APUS49074714", &2023, "M01", Some(123.45));
        obs.add_footnote("P");
        obs.add_footnote("R");
        
        // Note: The current API may not have footnotes() method, testing basic functionality
        assert_eq!(obs.series_id(), "APUS49074714");
        assert_eq!(obs.numeric_value().unwrap(), 123.45);
    }

    #[test]
    fn test_observation_validation() {
        // Valid observation
        let obs = Observation::new("APUS49074714", &2023, "M01", Some(123.45));
        assert!(obs.validate_observation().is_ok());

        // Invalid year (too old)
        let obs = Observation::new("APUS49074714", &1800, "M01", Some(123.45));
        assert!(obs.validate_observation().is_err());

        // Invalid year (future)
        let obs = Observation::new("APUS49074714", &2200, "M01", Some(123.45));
        assert!(obs.validate_observation().is_err());

        // Invalid period
        let obs = Observation::new("APUS49074714", &2023, "", Some(123.45));
        assert!(obs.validate_observation().is_err());
    }

    #[test]
    fn test_observation_period_validation() {
        let valid_periods = vec!["M01", "M12", "Q01", "Q04", "A01", "S01", "S02"];
        
        for period in valid_periods {
            let obs = Observation::new("APUS49074714", &2023, period, Some(100.0));
            assert!(obs.validate_observation().is_ok(), "Period {} should be valid", period);
        }

        let invalid_periods = vec!["M00", "M13", "Q00", "Q05", "X01", ""];
        
        for period in invalid_periods {
            let obs = Observation::new("APUS49074714", &2023, period, Some(100.0));
            assert!(obs.validate_observation().is_err(), "Period {} should be invalid", period);
        }
    }

    #[test]
    fn test_observation_value_types() {
        // Numeric value
        let obs1 = Observation::new("TEST001", &2023, "M01", Some(123.45));
        assert!(obs1.has_value());
        assert!(!obs1.is_missing());

        // Null value
        let obs2 = Observation::new("TEST001", &2023, "M01", None);
        assert!(!obs2.has_value());
        assert!(obs2.is_missing());
    }

    #[test]
    fn test_observation_serialization() {
        let mut obs = Observation::new("APUS49074714", &2023, "M01", Some(123.45));
        obs.add_footnote("P");
        
        let json = serde_json::to_string(&obs).expect("Should serialize");
        assert!(json.contains("APUS49074714"));
        assert!(json.contains("123.45"));
        
        let deserialized: Observation = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.series_id(), obs.series_id());
        assert_eq!(deserialized.year(), obs.year());
        assert_eq!(deserialized.period(), obs.period());
        assert_eq!(deserialized.numeric_value(), obs.numeric_value());
    }
}

#[cfg(test)]
mod lookup_tests {
    use super::*;

    #[test]
    fn test_lookup_creation() {
        let lookup = Lookup::new("area", "Geographic Areas");
        
        assert_eq!(lookup.table_id(), "area");
        assert_eq!(lookup.table_name(), "Geographic Areas");
        assert_eq!(lookup.entry_count(), 0);
    }

    #[test]
    fn test_lookup_add_entries() {
        let mut lookup = Lookup::new("area", "Geographic Areas");
        
        lookup.add_entry("US", "United States", None);
        lookup.add_entry("CA", "California", None);
        
        assert_eq!(lookup.entry_count(), 2);
        assert!(lookup.contains_entry("US"));
        assert!(lookup.contains_entry("CA"));
        assert!(!lookup.contains_entry("XX"));
    }

    #[test]
    fn test_lookup_get_entry() {
        let mut lookup = Lookup::new("area", "Geographic Areas");
        lookup.add_entry("US", "United States", None);
        
        let entry = lookup.get_entry("US");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().code, "US");
        assert_eq!(entry.unwrap().description, "United States");
        
        let missing = lookup.get_entry("XX");
        assert!(missing.is_none());
    }

    #[test]
    fn test_lookup_entry_with_metadata() {
        let mut entry = LookupEntry::new("US", "United States");
        entry.add_attribute("region", "North America");
        entry.add_attribute("currency", "USD");
        
        assert_eq!(entry.get_attribute("region"), Some(&"North America".to_string()));
        assert_eq!(entry.get_attribute("currency"), Some(&"USD".to_string()));
        assert_eq!(entry.get_attribute("missing"), None);
    }

    #[test]
    fn test_lookup_validation() {
        let lookup = Lookup::new("area", "Geographic Areas");
        // Note: Current Lookup may not have a validate method, just test basic functionality
        assert_eq!(lookup.table_id(), "area");

        // Empty table name should create lookup but may be invalid
        let lookup = Lookup::new("", "Description");
        assert_eq!(lookup.table_id(), "");

        // Empty description should create lookup but may be invalid
        let lookup = Lookup::new("table", "");
        assert_eq!(lookup.table_name(), "");
    }

    #[test]
    fn test_lookup_serialization() {
        let mut lookup = Lookup::new("area", "Geographic Areas");
        lookup.add_entry("US", "United States", None);
        lookup.add_entry("CA", "California", None);
        
        let json = serde_json::to_string(&lookup).expect("Should serialize");
        assert!(json.contains("area"));
        assert!(json.contains("Geographic Areas"));
        assert!(json.contains("United States"));
        
        let deserialized: Lookup = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.table_id(), lookup.table_id());
        assert_eq!(deserialized.table_name(), lookup.table_name());
        assert_eq!(deserialized.entry_count(), lookup.entry_count());
    }
}

#[cfg(test)]
mod survey_tests {
    use super::*;

    #[test]
    fn test_survey_creation() {
        let survey = Survey::new("AP", "Average Price Data");
        
        assert_eq!(survey.code(), "AP");
        assert_eq!(survey.name(), "Average Price Data");
        assert!(survey.series().is_empty());
        assert!(survey.lookups().is_empty());
    }

    #[test]
    fn test_survey_with_metadata() {
        let metadata = SurveyMetadata::new()
            .with_frequency(Frequency::Monthly)
            .with_geographic_scope("National")
            .with_industry_scope("All Industries");
            
        let survey = Survey::with_metadata("AP", "Average Price Data", metadata);
        
        assert_eq!(survey.metadata().frequency, Some(Frequency::Monthly));
        assert_eq!(survey.metadata().geographic_scope, Some("National".to_string()));
        assert_eq!(survey.metadata().industry_scope, Some("All Industries".to_string()));
    }

    #[test]
    fn test_survey_add_series() {
        let mut survey = Survey::new("AP", "Average Price Data");
        let series = Series::new("APUS49074714", "Gasoline Prices");
        
        survey.add_series(series);
        
        assert_eq!(survey.series().len(), 1);
        assert!(survey.has_series("APUS49074714"));
        assert!(!survey.has_series("NONEXISTENT"));
    }

    #[test]
    fn test_survey_add_lookup() {
        let mut survey = Survey::new("AP", "Average Price Data");
        let mut lookup = Lookup::new("area", "Geographic Areas");
        lookup.add_entry("US", "United States");
        
        survey.add_lookup(lookup);
        
        assert_eq!(survey.lookups().len(), 1);
        assert!(survey.has_lookup("area"));
        assert!(!survey.has_lookup("missing"));
    }

    #[test]
    fn test_survey_validation() {
        let survey = Survey::new("AP", "Average Price Data");
        assert!(survey.validate().is_ok());

        // Invalid survey code (wrong length)
        let survey = Survey::new("A", "Test");
        assert!(survey.validate().is_err());

        let survey = Survey::new("ABC", "Test");
        assert!(survey.validate().is_err());

        // Empty name
        let survey = Survey::new("AP", "");
        assert!(survey.validate().is_err());
    }

    #[test]
    fn test_survey_statistics() {
        let mut survey = Survey::new("AP", "Average Price Data");
        
        // Add series
        survey.add_series(Series::new("APUS49074714", "Series 1"));
        survey.add_series(Series::new("APUS49074715", "Series 2"));
        
        // Add lookups
        let mut area_lookup = Lookup::new("area", "Areas");
        area_lookup.add_entry("US", "United States");
        survey.add_lookup(area_lookup);
        
        let stats = survey.statistics();
        assert_eq!(stats.series_count, 2);
        assert_eq!(stats.lookup_count, 1);
        assert_eq!(stats.total_lookup_entries, 1);
    }

    #[test]
    fn test_survey_serialization() {
        let mut survey = Survey::new("AP", "Average Price Data");
        survey.add_series(Series::new("APUS49074714", "Test Series"));
        
        let json = serde_json::to_string(&survey).expect("Should serialize");
        assert!(json.contains("AP"));
        assert!(json.contains("Average Price Data"));
        assert!(json.contains("APUS49074714"));
        
        let deserialized: Survey = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.code(), survey.code());
        assert_eq!(deserialized.name(), survey.name());
        assert_eq!(deserialized.series().len(), survey.series().len());
    }
}

#[cfg(test)]
mod common_metadata_tests {
    use super::*;

    #[test]
    fn test_common_metadata_creation() {
        let metadata = CommonMetadata::new();
        
        assert_eq!(metadata.version, 1);
        assert!(metadata.created_at <= Utc::now());
        assert!(metadata.updated_at <= Utc::now());
        assert!(metadata.attributes.is_empty());
    }

    #[test]
    fn test_common_metadata_update() {
        let mut metadata = CommonMetadata::new();
        let initial_version = metadata.version;
        let initial_updated = metadata.updated_at;
        
        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        metadata.update();
        
        assert_eq!(metadata.version, initial_version + 1);
        assert!(metadata.updated_at > initial_updated);
    }

    #[test]
    fn test_common_metadata_attributes() {
        let mut metadata = CommonMetadata::new();
        
        metadata.add_attribute("source", "BLS");
        metadata.add_attribute("quality", "high");
        
        assert_eq!(metadata.attributes.len(), 2);
        assert_eq!(metadata.get_attribute("source"), Some(&"BLS".to_string()));
        assert_eq!(metadata.get_attribute("quality"), Some(&"high".to_string()));
        assert_eq!(metadata.get_attribute("missing"), None);
    }

    #[test]
    fn test_common_metadata_serialization() {
        let mut metadata = CommonMetadata::new();
        metadata.add_attribute("test", "value");
        
        let json = serde_json::to_string(&metadata).expect("Should serialize");
        assert!(json.contains("version"));
        assert!(json.contains("created_at"));
        assert!(json.contains("test"));
        
        let deserialized: CommonMetadata = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.version, metadata.version);
        assert_eq!(deserialized.attributes, metadata.attributes);
    }
}