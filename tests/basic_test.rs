/// Very basic test to verify core functionality without complex dependencies
/// This bypasses the full pipeline and just tests basic data structures

use rusty::data::model::{Series, Observation, Lookup};
use rusty::config::model::SurveyConfig;
use rusty::error::types::{DataError, ProcessingError};

#[test]
fn test_data_model_creation() {
    // Test Series creation
    let series = Series::new("TEST001".to_string())
        .with_title("Test Series")
        .with_area_code("US");
    
    assert_eq!(series.series_id(), "TEST001");
    assert_eq!(series.title().unwrap(), "Test Series");
    assert_eq!(series.area_code().unwrap(), "US");
    println!("✅ Series creation test passed!");
}

#[test]
fn test_observation_creation() {
    // Test Observation creation
    let obs = Observation::new("TEST001".to_string(), 2024, "M01")
        .with_value(100.5);
    
    assert_eq!(obs.series_id(), "TEST001");
    assert_eq!(obs.year(), 2024);
    assert_eq!(obs.period(), "M01");
    assert_eq!(obs.value().unwrap(), 100.5);
    println!("✅ Observation creation test passed!");
}

#[test]
fn test_lookup_creation() {
    // Test Lookup creation
    let lookup = Lookup::new("CODE001".to_string())
        .with_name("Test Name");
    
    assert_eq!(lookup.code(), "CODE001");
    assert_eq!(lookup.name().unwrap(), "Test Name");
    println!("✅ Lookup creation test passed!");
}

#[test]
fn test_config_creation() {
    // Test basic config creation
    let config = SurveyConfig::default();
    assert!(!config.overview.survey_code.is_empty());
    println!("✅ Config creation test passed!");
}

#[test]
fn test_error_creation() {
    // Test error creation
    let data_error = DataError::io_error("Test IO error".to_string());
    let proc_error = ProcessingError::UnsupportedDataType("Test unsupported".to_string());
    let not_impl_error = ProcessingError::NotImplemented("Test not implemented".to_string());
    
    // Just verify they can be created without panicking
    println!("✅ Error creation test passed!");
}

#[test]
fn test_series_builder_pattern() {
    // Test Series builder pattern
    let series = Series::new("TEST002".to_string())
        .with_title("Advanced Test Series")
        .with_area_code("CA")
        .with_item_code("ITEM123")
        .with_units("Dollars");
    
    assert_eq!(series.series_id(), "TEST002");
    assert_eq!(series.title().unwrap(), "Advanced Test Series");
    assert_eq!(series.area_code().unwrap(), "CA");
    assert_eq!(series.item_code().unwrap(), "ITEM123");
    assert_eq!(series.units().unwrap(), "Dollars");
    println!("✅ Series builder pattern test passed!");
}

#[test] 
fn test_observation_with_footnotes() {
    // Test Observation with footnotes
    let obs = Observation::new("TEST002".to_string(), 2024, "Q1")
        .with_value(250.75)
        .with_footnote_codes(vec!["P".to_string(), "R".to_string()]);
    
    assert_eq!(obs.series_id(), "TEST002");
    assert_eq!(obs.period(), "Q1");
    assert_eq!(obs.value().unwrap(), 250.75);
    assert_eq!(obs.footnote_codes().len(), 2);
    println!("✅ Observation with footnotes test passed!");
}

#[test]
fn test_lookup_with_description() {
    // Test Lookup with description
    let lookup = Lookup::new("STATE01".to_string())
        .with_name("Alabama")
        .with_description("State lookup for Alabama");
    
    assert_eq!(lookup.code(), "STATE01");
    assert_eq!(lookup.name().unwrap(), "Alabama");
    assert_eq!(lookup.description().unwrap(), "State lookup for Alabama");
    println!("✅ Lookup with description test passed!");
}