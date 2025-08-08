use rusty::config::model::SurveyConfig;
/// Very basic test to verify core functionality without complex dependencies
/// This bypasses the full pipeline and just tests basic data structures
use rusty::data::model::{Lookup, Observation, Series};
use rusty::error::types::{DataError, ProcessingError};

#[test]
fn test_data_model_creation() {
    // Test Series creation
    let series = Series::new("TEST001", "Test Series");
    
    assert_eq!(series.id(), "TEST001");
    assert_eq!(series.title(), "Test Series");
    println!("✅ Series creation test passed!");
}

#[test]
fn test_observation_creation() {
    // Test Observation creation
    let obs = Observation::new("TEST001", &2024, "M01", Some(100.5));
    
    assert_eq!(obs.series_id(), "TEST001");
    assert_eq!(obs.year(), 2024);
    assert_eq!(obs.period(), "M01");
    assert_eq!(obs.numeric_value().unwrap(), 100.5);
    println!("✅ Observation creation test passed!");
}

#[test]
fn test_lookup_creation() {
    // Test Lookup creation - needs table_id and table_name
    let lookup = Lookup::new("CODE001", "Test Lookup Table");
    
    // Test that we can add an entry and retrieve it
    let mut lookup = lookup;
    lookup.add_entry("ENTRY001", "Test Entry", None);
    
    assert!(lookup.contains_entry("ENTRY001"));
    assert_eq!(lookup.entry_count(), 1);
    println!("✅ Lookup creation test passed!");
}

#[test]
fn test_config_creation() {
    // Test basic config creation
    let config = SurveyConfig::new("TEST");
    assert_eq!(config.overview.survey.code, "TEST");
    println!("✅ Config creation test passed!");
}

#[test]
fn test_error_creation() {
    // Test error creation using public constructors
    let proc_error = ProcessingError::UnsupportedDataType("Test unsupported".to_string());
    let not_impl_error = ProcessingError::NotImplemented("Test not implemented".to_string());
    
    // Just verify they can be created without panicking
    println!("✅ Error creation test passed!");
}

#[test]
fn test_series_with_metadata() {
    // Test Series creation with proper API
    let series = Series::new("TEST002", "Advanced Test Series");
    
    assert_eq!(series.id(), "TEST002");
    assert_eq!(series.title(), "Advanced Test Series");
    println!("✅ Series with metadata test passed!");
}

#[test] 
fn test_observation_with_value() {
    // Test Observation with value using proper API
    let mut obs = Observation::new("TEST002", &2024, "Q1", Some(250.75));
    obs.add_footnote("P");
    obs.add_footnote("R");
    
    assert_eq!(obs.series_id(), "TEST002");
    assert_eq!(obs.period(), "Q1");
    assert_eq!(obs.numeric_value().unwrap(), 250.75);
    println!("✅ Observation with value test passed!");
}

#[test]
fn test_lookup_operations() {
    // Test Lookup operations with proper API
    let mut lookup = Lookup::new("STATE01", "State Lookup Table");
    lookup.add_entry("AL", "Alabama", None);
    lookup.add_entry("CA", "California", None);
    
    assert!(lookup.contains_entry("AL"));
    assert!(lookup.contains_entry("CA"));
    assert_eq!(lookup.entry_count(), 2);
    
    let entry = lookup.get_entry("AL").unwrap();
    assert_eq!(entry.code, "AL");
    assert_eq!(entry.description, "Alabama");
    println!("✅ Lookup operations test passed!");
}
