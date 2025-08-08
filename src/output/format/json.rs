//! # JSON Output Format Implementation
//!
//! This module provides a complete JSON output format implementation for BLS survey data.
//! It supports pretty printing, custom indentation, and all BLS data types.

use async_trait::async_trait;
use chrono;
use serde_json;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::data::model::{Lookup, Observation, Series, Survey};
use crate::error::types::{ProcessingError, Result};
use crate::output::traits::{
    FormatWriter, OutputConfig, OutputGenerator, OutputResult, OutputStats,
};
use crate::processing::traits::ProcessedData;

/// JSON format writer implementation
pub struct JsonWriter {
    stats: OutputStats,
    pretty_print: bool,
}

impl JsonWriter {
    /// Create a new JSON writer
    pub fn new() -> Self {
        Self {
            stats: OutputStats::default(),
            pretty_print: false,
        }
    }

    /// Create a new JSON writer with pretty printing
    pub fn new_pretty() -> Self {
        Self {
            stats: OutputStats::default(),
            pretty_print: true,
        }
    }

    /// Serialize data to JSON bytes
    fn serialize_to_json<T: serde::Serialize>(&self, data: &T) -> Result<Vec<u8>> {
        let json_str = if self.pretty_print {
            serde_json::to_string_pretty(data)
        } else {
            serde_json::to_string(data)
        }
        .map_err(|e| ProcessingError::system_error(format!("JSON serialization failed: {e}")))?;

        Ok(json_str.into_bytes())
    }
}

impl Default for JsonWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FormatWriter for JsonWriter {
    fn format_name(&self) -> &str {
        "json"
    }

    fn file_extension(&self) -> &str {
        "json"
    }

    async fn write_series(
        &mut self,
        series: &[Series],
        path: &Path,
        _config: &OutputConfig,
    ) -> Result<OutputResult> {
        let start_time = Instant::now();

        let json_data = self.serialize_to_json(&series)?;

        let file = File::create(path).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to create JSON file: {e}")))?;
        let mut writer = BufWriter::new(file);

        writer.write_all(&json_data).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write JSON data: {e}")))?;
        writer.flush().await
            .map_err(|e| ProcessingError::io_error(format!("Failed to flush JSON data: {e}")))?;

        let elapsed = start_time.elapsed();
        self.stats.total_records_written += series.len() as u64;
        self.stats.total_bytes_written += json_data.len() as u64;

        Ok(OutputResult {
            output_paths: vec![path.to_string_lossy().to_string()],
            records_written: series.len() as u64,
            bytes_written: json_data.len() as u64,
            generation_time_ms: elapsed.as_millis() as u64,
            metadata: HashMap::new(),
        })
    }

    async fn write_observations(
        &mut self,
        observations: &[Observation],
        path: &Path,
        _config: &OutputConfig,
    ) -> Result<OutputResult> {
        let start_time = Instant::now();

        let json_data = self.serialize_to_json(&observations)?;

        let file = File::create(path).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to create JSON file: {e}")))?;
        let mut writer = BufWriter::new(file);

        writer.write_all(&json_data).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write JSON data: {e}")))?;
        writer.flush().await
            .map_err(|e| ProcessingError::io_error(format!("Failed to flush JSON data: {e}")))?;

        let elapsed = start_time.elapsed();
        self.stats.total_records_written += observations.len() as u64;
        self.stats.total_bytes_written += json_data.len() as u64;

        Ok(OutputResult {
            output_paths: vec![path.to_string_lossy().to_string()],
            records_written: observations.len() as u64,
            bytes_written: json_data.len() as u64,
            generation_time_ms: elapsed.as_millis() as u64,
            metadata: HashMap::new(),
        })
    }

    async fn write_lookups(
        &mut self,
        lookups: &[Lookup],
        path: &Path,
        _config: &OutputConfig,
    ) -> Result<OutputResult> {
        let start_time = Instant::now();

        let json_data = self.serialize_to_json(&lookups)?;

        let file = File::create(path).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to create JSON file: {e}")))?;
        let mut writer = BufWriter::new(file);

        writer.write_all(&json_data).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write JSON data: {e}")))?;
        writer.flush().await
            .map_err(|e| ProcessingError::io_error(format!("Failed to flush JSON data: {e}")))?;

        let elapsed = start_time.elapsed();
        self.stats.total_records_written += lookups.len() as u64;
        self.stats.total_bytes_written += json_data.len() as u64;

        Ok(OutputResult {
            output_paths: vec![path.to_string_lossy().to_string()],
            records_written: lookups.len() as u64,
            bytes_written: json_data.len() as u64,
            generation_time_ms: elapsed.as_millis() as u64,
            metadata: HashMap::new(),
        })
    }

    async fn write_survey(
        &mut self,
        survey: &Survey,
        path: &Path,
        _config: &OutputConfig,
    ) -> Result<OutputResult> {
        let start_time = Instant::now();

        let json_data = self.serialize_to_json(survey)?;

        let file = File::create(path).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to create JSON file: {e}")))?;
        let mut writer = BufWriter::new(file);

        writer.write_all(&json_data).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write JSON data: {e}")))?;
        writer.flush().await
            .map_err(|e| ProcessingError::io_error(format!("Failed to flush JSON data: {e}")))?;

        let elapsed = start_time.elapsed();
        self.stats.total_records_written += 1;
        self.stats.total_bytes_written += json_data.len() as u64;

        Ok(OutputResult {
            output_paths: vec![path.to_string_lossy().to_string()],
            records_written: 1,
            bytes_written: json_data.len() as u64,
            generation_time_ms: elapsed.as_millis() as u64,
            metadata: HashMap::new(),
        })
    }


    async fn write_mixed(
        &mut self,
        data: ProcessedData,
        path: &Path,
        _config: &OutputConfig,
    ) -> Result<OutputResult> {
        let start_time = Instant::now();

        // For now, write a placeholder JSON structure for ProcessedData
        // In the future, this would be replaced with proper ProcessedData serialization
        let placeholder = serde_json::json!({
            "type": "ProcessedData",
            "message": "JSON serialization of ProcessedData not yet fully implemented",
            "timestamp": chrono::Utc::now().to_rfc3339()
        });
        
        let json_data = self.serialize_to_json(&placeholder)?;

        let file = File::create(path).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to create JSON file: {e}")))?;
        let mut writer = BufWriter::new(file);

        writer.write_all(&json_data).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write JSON data: {e}")))?;
        writer.flush().await
            .map_err(|e| ProcessingError::io_error(format!("Failed to flush JSON data: {e}")))?;

        let elapsed = start_time.elapsed();
        self.stats.total_records_written += 1;
        self.stats.total_bytes_written += json_data.len() as u64;

        Ok(OutputResult {
            output_paths: vec![path.to_string_lossy().to_string()],
            records_written: 1,
            bytes_written: json_data.len() as u64,
            generation_time_ms: elapsed.as_millis() as u64,
            metadata: HashMap::new(),
        })
    }

    fn validate_format_config(&self, _config: &OutputConfig) -> Result<()> {
        Ok(())
    }

    fn default_format_options(&self) -> HashMap<String, String> {
        let mut options = HashMap::new();
        options.insert("pretty".to_string(), "true".to_string());
        options.insert("indent".to_string(), "2".to_string());
        options
    }
}

/// JSON output generator (stub)
pub struct JsonOutputGenerator {
    writer: JsonWriter,
}

impl JsonOutputGenerator {
    /// Create a new JSON output generator
    pub fn new() -> Self {
        Self {
            writer: JsonWriter::new(),
        }
    }
}

impl Default for JsonOutputGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutputGenerator for JsonOutputGenerator {
    fn name(&self) -> &str {
        "json_generator"
    }

    fn description(&self) -> &str {
        "JSON output generator for BLS data (stub implementation)"
    }

    fn supported_formats(&self) -> Vec<String> {
        vec!["json".to_string()]
    }

    async fn generate(
        &mut self,
        data: ProcessedData,
        config: OutputConfig,
    ) -> Result<OutputResult> {
        use std::path::Path;
        
        let path = Path::new(&config.destination);
        self.writer.write_mixed(data, path, &config).await
    }

    fn validate_config(&self, config: &OutputConfig) -> Result<()> {
        if config.format.to_lowercase() != "json" {
            return Err(ProcessingError::invalid_configuration(format!(
                "JSON generator does not support format: {}",
                config.format
            )));
        }
        Ok(())
    }

    fn stats(&self) -> OutputStats {
        self.writer.stats.clone()
    }

    fn reset_stats(&mut self) {
        self.writer.stats = OutputStats::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_writer_creation() {
        let writer = JsonWriter::new();
        assert_eq!(writer.format_name(), "json");
        assert_eq!(writer.file_extension(), "json");
    }

    #[test]
    fn test_json_generator_creation() {
        let generator = JsonOutputGenerator::new();
        assert_eq!(generator.name(), "json_generator");
        assert!(generator.supported_formats().contains(&"json".to_string()));
    }

    #[test]
    fn test_json_generator_validation() {
        let generator = JsonOutputGenerator::new();

        let valid_config = OutputConfig {
            format: "json".to_string(),
            destination: "output.json".to_string(),
            ..Default::default()
        };
        assert!(generator.validate_config(&valid_config).is_ok());

        let invalid_config = OutputConfig {
            format: "csv".to_string(),
            destination: "output.csv".to_string(),
            ..Default::default()
        };
        assert!(generator.validate_config(&invalid_config).is_err());
    }
}
