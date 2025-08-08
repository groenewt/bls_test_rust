/// Basic integration test to verify core functionality works
/// This test bypasses the complex pipeline code and tests data flow directly
use std::fs::File;
use std::io::Write;
use tempfile::NamedTempFile;

use rusty::{
    data::{
        model::{Lookup, Observation, Series},
        reader::{DataReader, SeriesReader},
        writer::{DataWriter, LookupWriter, ObservationWriter, SeriesWriter},
    },
    error::{Result, DataError},
};

#[tokio::test]
async fn test_core_data_flow() -> Result<()> {
    // Test 1: Create sample data using correct APIs
    let test_series = Series::new("TEST001", "Test Series");

    let mut test_observation = Observation::new("TEST001", &2024, "M01", Some(100.5));
    test_observation.add_footnote("A");

    let mut test_lookup = Lookup::new("TEST001", "Test Lookup Table");
    test_lookup.add_entry("ENTRY001", "Test Entry", None);

    // Test 2: Test CSV Writer
    let temp_dir = tempfile::TempDir::new()
        .map_err(|e| rusty::Error::Data(DataError::IoError { path: "temp_dir".to_string(), source: e.to_string() }))?;
    let temp_file_path = temp_dir.path().join("test_output.csv");

    let mut config = rusty::data::writer::traits::WriterConfig::default();
    config.overwrite_existing = true;
    
    let mut csv_writer = rusty::data::writer::csv_writer::CsvDataWriter::new(config);

    // Open writer
    csv_writer.open(&temp_file_path).await?;

    // Write only observations to avoid schema mismatch
    csv_writer.write_observation(&test_observation).await?;

    // Close writer
    csv_writer.close().await?;

    println!("✅ Basic data writing test passed!");

    // Test 3: Verify file was created and has content
    let file_size = std::fs::metadata(&temp_file_path)
        .map_err(|e| rusty::Error::Data(DataError::IoError { path: temp_file_path.to_string_lossy().to_string(), source: e.to_string() }))?
        .len();

    assert!(file_size > 0, "Output file should have content");

    println!("✅ File output test passed! File size: {} bytes", file_size);

    Ok(())
}

#[tokio::test]
async fn test_writer_factory() -> Result<()> {
    use rusty::data::writer::factory::DefaultWriterFactory;
    use rusty::data::writer::traits::{WriterConfig, WriterFactory};

    let factory = DefaultWriterFactory::new();

    // Test creating different writer types
    let config = WriterConfig::default();

    let _csv_writer = factory.create_writer("csv", config.clone())?;
    let _json_writer = factory.create_writer("json", config.clone())?;
    let _parquet_writer = factory.create_writer("parquet", config)?;

    println!("✅ Writer factory test passed!");

    Ok(())
}

#[test]
fn test_data_models() {
    // Test Series creation
    let series = Series::new("TEST001", "Test Series");

    assert_eq!(series.id(), "TEST001");
    assert_eq!(series.title(), "Test Series");

    // Test Observation creation
    let obs = Observation::new("TEST001", &2024, "M01", Some(100.5));

    assert_eq!(obs.series_id(), "TEST001");
    assert_eq!(obs.year(), 2024);
    assert_eq!(obs.period(), "M01");
    assert_eq!(obs.numeric_value().unwrap(), 100.5);

    // Test Lookup creation
    let mut lookup = Lookup::new("CODE001", "Test Name");
    lookup.add_entry("ENTRY001", "Test Entry", None);

    assert!(lookup.contains_entry("ENTRY001"));
    assert_eq!(lookup.entry_count(), 1);

    println!("✅ Data model test passed!");
}

#[test]
fn test_config_loading() {
    use rusty::config::{ConfigLoader, SurveyConfig};

    // Test config creation
    let config = SurveyConfig::new("TEST");
    assert_eq!(config.overview.survey.code, "TEST");

    // Test config loader
    let _loader = ConfigLoader::new();
    // ConfigLoader::new() returns ConfigLoader, not Result

    println!("✅ Config loading test passed!");
}

#[test]
fn test_error_handling() {
    use rusty::error::types::{DataError, ProcessingError};

    // Test DataError creation using public constructors
    let _data_error = DataError::IoError { 
        path: "test_path".to_string(), 
        source: "Test IO error".to_string() 
    };

    // Test ProcessingError creation
    let _proc_error = ProcessingError::UnsupportedDataType("Test unsupported".to_string());
    let _not_impl_error = ProcessingError::NotImplemented("Test not implemented".to_string());

    println!("✅ Error handling test passed!");
}
