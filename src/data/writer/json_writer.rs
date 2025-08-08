//! JSON writer implementation for BLS data processing
//!
//! This module provides a concrete implementation of the writer traits for
//! JSON format output. It supports structured data, custom serialization,
//! and both compact and pretty-printed formats.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;
use async_trait::async_trait;
use serde_json::{json, Value, Map};
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::data::writer::traits::{
    DataWriter, SeriesWriter, ObservationWriter, LookupWriter, SurveyWriter,
    StreamingWriter, CompressedWriter, WriterConfig, WriteStats, CompressionInfo,
};
use crate::error::types::{DataError, Result};
use crate::utils::validation::BLSValidationRules;

/// JSON writer implementation
pub struct JsonDataWriter {
    config: WriterConfig,
    stats: WriteStats,
    current_file: Option<PathBuf>,
    writer: Option<Mutex<Box<dyn Write + Send>>>,
    validation_rules: BLSValidationRules,
    data_buffer: Vec<Value>,
    is_array_format: bool,
    first_record: bool,
    compression_info: Option<CompressionInfo>,
}

impl JsonDataWriter {
    /// Create a new JSON writer with the given configuration
    pub fn new(config: WriterConfig) -> Self {
        Self {
            config,
            stats: WriteStats::default(),
            current_file: None,
            writer: None,
            validation_rules: BLSValidationRules::default(),
            data_buffer: Vec::new(),
            is_array_format: true, // Default to array format
            first_record: true,
            compression_info: None,
        }
    }

    /// Create a JSON writer with appropriate compression if configured
    fn create_writer(&self, file: File) -> Result<Mutex<Box<dyn Write + Send>>> {
        let writer: Box<dyn Write + Send> = if self.config.compress_output {
            let compression = Compression::new(self.config.compression_level as u32);
            Box::new(GzEncoder::new(BufWriter::new(file), compression))
        } else {
            Box::new(BufWriter::new(file))
        };

        Ok(Mutex::new(writer))
    }

    /// Set whether to use array format (true) or line-delimited JSON (false)
    pub fn set_array_format(&mut self, array_format: bool) {
        self.is_array_format = array_format;
    }

    /// Convert a series to JSON value
    fn series_to_json(&self, series: &Series) -> Value {
        let mut obj = Map::new();
        obj.insert("series_id".to_string(), json!(series.id()));
        obj.insert("title".to_string(), json!(series.title()));
        obj.insert("survey_code".to_string(), json!(series.survey_code()));
        obj.insert("frequency".to_string(), json!(series.frequency().to_string()));
        
        if let Some(area) = series.area() {
            obj.insert("area_code".to_string(), json!(area.code));
            obj.insert("area_name".to_string(), json!(area.name));
        }
        if let Some(item) = series.item() {
            obj.insert("item_code".to_string(), json!(item.code));
            obj.insert("item_name".to_string(), json!(item.name));
        }
        if let Some(unit) = series.unit() {
            obj.insert("units".to_string(), json!(unit.name));
        }
        
        // Add metadata fields
        obj.insert("is_active".to_string(), json!(series.is_active()));

        Value::Object(obj)
    }

    /// Convert an observation to JSON value
    fn observation_to_json(&self, observation: &Observation) -> Value {
        let mut obj = Map::new();
        obj.insert("series_id".to_string(), json!(observation.series_id()));
        obj.insert("year".to_string(), json!(observation.year()));
        obj.insert("period".to_string(), json!(observation.period()));
        
        if let Some(value) = observation.numeric_value() {
            // Format float with configured precision
            let formatted_value = format!("{:.precision$}", value, precision = self.config.float_precision);
            obj.insert("value".to_string(), json!(formatted_value.parse::<f64>().unwrap_or(value)));
        } else {
            obj.insert("value".to_string(), Value::Null);
        }
        
        // Add data quality and other metadata
        obj.insert("quality".to_string(), json!(observation.quality()));
        obj.insert("has_value".to_string(), json!(observation.has_value()));
        obj.insert("is_missing".to_string(), json!(observation.is_missing()));

        Value::Object(obj)
    }

    /// Convert a lookup to JSON value
    fn lookup_to_json(&self, lookup: &Lookup) -> Value {
        let mut obj = Map::new();
        obj.insert("table_id".to_string(), json!(lookup.table_id));
        obj.insert("table_name".to_string(), json!(lookup.table_name));
        obj.insert("survey_code".to_string(), json!(lookup.survey_code));
        obj.insert("entry_count".to_string(), json!(lookup.entry_count()));
        
        // Convert entries to a JSON object
        let mut entries_obj = Map::new();
        for (code, entry) in &lookup.entries {
            let mut entry_obj = Map::new();
            entry_obj.insert("code".to_string(), json!(entry.code));
            entry_obj.insert("description".to_string(), json!(entry.description));
            entry_obj.insert("active".to_string(), json!(entry.active));
            if let Some(parent_code) = &entry.parent_code {
                entry_obj.insert("parent_code".to_string(), json!(parent_code));
            }
            entries_obj.insert(code.clone(), Value::Object(entry_obj));
        }
        obj.insert("entries".to_string(), Value::Object(entries_obj));

        Value::Object(obj)
    }

    /// Convert a survey to JSON value
    fn survey_to_json(&self, survey: &Survey) -> Value {
        let mut obj = Map::new();
        obj.insert("survey_code".to_string(), json!(survey.code()));
        obj.insert("survey_name".to_string(), json!(survey.name()));
        if let Some(description) = survey.description() {
            obj.insert("description".to_string(), json!(description));
        }

        Value::Object(obj)
    }

    /// Write JSON value to the writer
    fn write_json_value(&mut self, value: &Value) -> Result<u64> {
        if let Some(ref writer) = self.writer {
            let mut writer = writer.lock().unwrap();
            let json_string = if self.config.include_headers {
                // Pretty print for readability
                serde_json::to_string_pretty(value)
                    .map_err(|e| DataError::serialization_error(format!("Failed to serialize JSON: {}", e)))?
            } else {
                // Compact format
                serde_json::to_string(value)
                    .map_err(|e| DataError::serialization_error(format!("Failed to serialize JSON: {}", e)))?
            };

            let bytes_written = if self.is_array_format {
                // Array format: [item1, item2, ...]
                if self.first_record {
                    writer.write_all(b"[\n")?;
                    self.first_record = false;
                } else {
                    writer.write_all(b",\n")?;
                }
                writer.write_all(json_string.as_bytes())?;
                json_string.len() + 2 // +2 for comma and newline
            } else {
                // Line-delimited JSON format: one JSON object per line
                writer.write_all(json_string.as_bytes())?;
                writer.write_all(b"\n")?;
                json_string.len() + 1 // +1 for newline
            };

            Ok(bytes_written as u64)
        } else {
            Err(DataError::IoError {
                path: "unknown".to_string(),
                source: "No writer available".to_string(),
            }.into())
        }
    }

    /// Validate a record before writing
    fn validate_record<T>(&self, _record: &T, record_type: &str) -> Result<bool>
    where
        T: std::fmt::Debug,
    {
        if !self.config.validate_on_write {
            return Ok(true);
        }

        match record_type {
            "series" | "observation" | "lookup" | "survey" => Ok(true),
            _ => Err(DataError::ValidationError {
                message: format!("Unknown record type: {}", record_type),
                path: None,
                line: None,
            }.into()),
        }
    }

    /// Update statistics after writing records
    fn update_stats(&mut self, records_written: u64, bytes_written: u64, errors: u64, start_time: Instant) {
        self.stats.records_written += records_written;
        self.stats.bytes_written += bytes_written;
        self.stats.errors_encountered += errors;
        self.stats.write_time_ms += start_time.elapsed().as_millis() as u64;
    }

    /// Finalize the JSON output (close array if needed)
    fn finalize_output(&mut self) -> Result<()> {
        if let Some(ref writer) = self.writer {
            let mut writer = writer.lock().unwrap();
            if self.is_array_format && !self.first_record {
                writer.write_all(b"\n]")?;
            }
            writer.flush()?;
        }
        Ok(())
    }

    /// Calculate compression statistics
    fn calculate_compression_stats(&mut self, original_size: u64, compressed_size: u64) {
        let ratio = if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            1.0
        };

        self.compression_info = Some(CompressionInfo {
            algorithm: if self.config.compress_output {
                "gzip".to_string()
            } else {
                "none".to_string()
            },
            level: self.config.compression_level,
            original_size,
            compressed_size,
            ratio,
        });

        self.stats.compression_ratio = ratio;
    }
}

#[async_trait]
impl DataWriter for JsonDataWriter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn config(&self) -> &WriterConfig {
        &self.config
    }

    fn stats(&self) -> &WriteStats {
        &self.stats
    }

    fn reset_stats(&mut self) {
        self.stats = WriteStats::default();
        self.compression_info = None;
    }

    fn can_write(&self, path: &Path) -> Result<bool> {
        // Check if we can write to the directory
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Ok(false);
            }
        }

        // Check file extension
        if let Some(extension) = path.extension() {
            match extension.to_str() {
                Some("json") | Some("jsonl") => Ok(true),
                Some("gz") => {
                    // Check if it's a compressed JSON
                    if let Some(stem) = path.file_stem() {
                        if let Some(stem_str) = stem.to_str() {
                            Ok(stem_str.ends_with(".json") || stem_str.ends_with(".jsonl"))
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(false)
                    }
                }
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    async fn open(&mut self, path: &Path) -> Result<()> {
        if !self.can_write(path)? {
            return Err(DataError::unsupported_format(
                format!("Cannot write JSON to: {}", path.display())
            ).into());
        }

        // Check if file exists and we're not allowed to overwrite
        if path.exists() && !self.config.overwrite_existing {
            return Err(DataError::IoError {
                path: path.display().to_string(),
                source: "File already exists and overwrite is disabled".to_string(),
            }.into());
        }

        // Determine format based on file extension
        if let Some(extension) = path.extension() {
            if extension == "jsonl" {
                self.is_array_format = false;
            }
        }

        let file = File::create(path)
            .map_err(|e| DataError::IoError {
                path: path.display().to_string(),
                source: format!("Failed to create file: {}", e),
            })?;

        let writer = self.create_writer(file)?;
        self.writer = Some(writer);
        self.current_file = Some(path.to_path_buf());
        self.first_record = true;
        self.data_buffer.clear();
        self.reset_stats();

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.finalize_output()?;
        self.writer = None;
        self.current_file = None;
        self.first_record = true;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if let Some(ref writer) = self.writer {
            let mut writer = writer.lock().unwrap();
            writer.flush()
                .map_err(|e| DataError::IoError {
                    path: "unknown".to_string(),
                    source: format!("Failed to flush writer: {}", e),
                })?;
        }
        Ok(())
    }

    fn is_open(&self) -> bool {
        self.writer.is_some()
    }

    fn current_file(&self) -> Option<&Path> {
        self.current_file.as_deref()
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["json".to_string(), "jsonl".to_string()]
    }
}

#[async_trait]
impl SeriesWriter for JsonDataWriter {
    async fn write_series(&mut self, series: &Series) -> Result<()> {
        let start_time = Instant::now();
        
        self.validate_record(series, "series")?;
        let json_value = self.series_to_json(series);
        let bytes_written = self.write_json_value(&json_value)?;

        self.update_stats(1, bytes_written, 0, start_time);
        Ok(())
    }

    async fn write_series_batch(&mut self, series: &[Series]) -> Result<()> {
        let start_time = Instant::now();
        let mut total_bytes = 0;
        let mut errors = 0;

        for s in series {
            match self.validate_record(s, "series") {
                Ok(_) => {
                    let json_value = self.series_to_json(s);
                    match self.write_json_value(&json_value) {
                        Ok(bytes) => total_bytes += bytes,
                        Err(_) => errors += 1,
                    }
                }
                Err(_) => errors += 1,
            }
        }

        self.update_stats(series.len() as u64 - errors, total_bytes, errors, start_time);
        Ok(())
    }

    // Generic method write_all_series moved to extension trait

    // Generic method write_series_with_serializer moved to extension trait
}

#[async_trait]
impl ObservationWriter for JsonDataWriter {
    async fn write_observation(&mut self, observation: &Observation) -> Result<()> {
        let start_time = Instant::now();
        
        self.validate_record(observation, "observation")?;
        let json_value = self.observation_to_json(observation);
        let bytes_written = self.write_json_value(&json_value)?;

        self.update_stats(1, bytes_written, 0, start_time);
        Ok(())
    }

    async fn write_observations_batch(&mut self, observations: &[Observation]) -> Result<()> {
        let start_time = Instant::now();
        let mut total_bytes = 0;
        let mut errors = 0;

        for obs in observations {
            match self.validate_record(obs, "observation") {
                Ok(_) => {
                    let json_value = self.observation_to_json(obs);
                    match self.write_json_value(&json_value) {
                        Ok(bytes) => total_bytes += bytes,
                        Err(_) => errors += 1,
                    }
                }
                Err(_) => errors += 1,
            }
        }

        self.update_stats(observations.len() as u64 - errors, total_bytes, errors, start_time);
        Ok(())
    }

    // Generic method write_all_observations moved to extension trait

    async fn write_observations_for_series(&mut self, series_id: &str, observations: &[Observation]) -> Result<()> {
        let filtered_observations: Vec<&Observation> = observations
            .iter()
            .filter(|obs| obs.series_id() == series_id)
            .collect();

        let owned_observations: Vec<Observation> = filtered_observations
            .into_iter()
            .cloned()
            .collect();

        self.write_observations_batch(&owned_observations).await
    }

    // Generic method write_observations_with_serializer moved to extension trait
}

#[async_trait]
impl LookupWriter for JsonDataWriter {
    async fn write_lookup(&mut self, lookup: &Lookup) -> Result<()> {
        let start_time = Instant::now();
        
        self.validate_record(lookup, "lookup")?;
        let json_value = self.lookup_to_json(lookup);
        let bytes_written = self.write_json_value(&json_value)?;

        self.update_stats(1, bytes_written, 0, start_time);
        Ok(())
    }

    async fn write_lookups_batch(&mut self, lookups: &[Lookup]) -> Result<()> {
        let start_time = Instant::now();
        let mut total_bytes = 0;
        let mut errors = 0;

        for lookup in lookups {
            match self.validate_record(lookup, "lookup") {
                Ok(_) => {
                    let json_value = self.lookup_to_json(lookup);
                    match self.write_json_value(&json_value) {
                        Ok(bytes) => total_bytes += bytes,
                        Err(_) => errors += 1,
                    }
                }
                Err(_) => errors += 1,
            }
        }

        self.update_stats(lookups.len() as u64 - errors, total_bytes, errors, start_time);
        Ok(())
    }

    // Generic method write_all_lookups moved to extension trait

    // Generic method write_lookups_with_serializer moved to extension trait
}

#[async_trait]
impl SurveyWriter for JsonDataWriter {
    async fn write_survey(&mut self, survey: &Survey) -> Result<()> {
        let start_time = Instant::now();
        
        self.validate_record(survey, "survey")?;
        let json_value = self.survey_to_json(survey);
        let bytes_written = self.write_json_value(&json_value)?;

        self.update_stats(1, bytes_written, 0, start_time);
        Ok(())
    }

    // Generic method write_survey_with_format moved to extension trait
}

#[async_trait]
impl StreamingWriter for JsonDataWriter {
    async fn start_stream(&mut self) -> Result<()> {
        if self.is_array_format && self.writer.is_some() {
            if let Some(ref writer) = self.writer {
            let mut writer = writer.lock().unwrap();
                writer.write_all(b"[\n")?;
                self.first_record = false;
            }
        }
        Ok(())
    }

    async fn write_chunk(&mut self, data: &[u8]) -> Result<()> {
        if let Some(ref writer) = self.writer {
            let mut writer = writer.lock().unwrap();
            writer.write_all(data)?;
            self.stats.bytes_written += data.len() as u64;
        }
        Ok(())
    }

    // Generic method write_record moved to extension trait

    async fn end_stream(&mut self) -> Result<()> {
        if self.is_array_format && self.writer.is_some() {
            if let Some(ref writer) = self.writer {
            let mut writer = writer.lock().unwrap();
                writer.write_all(b"\n]")?;
            }
        }
        self.flush().await
    }
}

#[async_trait]
impl CompressedWriter for JsonDataWriter {
    fn set_compression_level(&mut self, level: u8) -> Result<()> {
        if level > 9 {
            return Err(DataError::invalid_configuration(
                "Compression level must be between 0 and 9".to_string()
            ).into());
        }
        self.config.compression_level = level;
        Ok(())
    }

    fn compression_level(&self) -> u8 {
        self.config.compression_level
    }

    fn compression_stats(&self) -> (u64, u64, f64) {
        if let Some(ref info) = self.compression_info {
            (info.original_size, info.compressed_size, info.ratio)
        } else {
            (0, 0, 1.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Read;

    #[tokio::test]
    async fn test_json_writer_creation() {
        let config = WriterConfig::default();
        let writer = JsonDataWriter::new(config);
        assert!(!writer.is_open());
        assert!(writer.current_file().is_none());
    }

    #[tokio::test]
    async fn test_can_write_json() {
        let config = WriterConfig::default();
        let writer = JsonDataWriter::new(config);
        
        let json_path = Path::new("test.json");
        assert!(writer.can_write(json_path).unwrap());
        
        let jsonl_path = Path::new("test.jsonl");
        assert!(writer.can_write(jsonl_path).unwrap());
        
        let invalid_path = Path::new("test.csv");
        assert!(!writer.can_write(invalid_path).unwrap());
    }

    #[tokio::test]
    async fn test_write_series() {
        let config = WriterConfig::default();
        let mut writer = JsonDataWriter::new(config);
        
        let temp_file = NamedTempFile::with_suffix(".json").unwrap();
        writer.open(temp_file.path()).await.unwrap();
        
        let series = Series::new(
            &"TEST001",
            &"Test Series"
        );
        
        writer.write_series(&series).await.unwrap();
        writer.close().await.unwrap();
        
        // Verify the file was written
        let mut file_content = String::new();
        let mut file = File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut file_content).unwrap();
        
        assert!(file_content.contains("TEST001"));
        assert!(file_content.contains("Test Series"));
        assert!(file_content.contains("AREA001"));
    }

    #[tokio::test]
    async fn test_array_vs_line_delimited() {
        let config = WriterConfig::default();
        
        // Test array format
        let mut array_writer = JsonDataWriter::new(config.clone());
        array_writer.set_array_format(true);
        
        let temp_file1 = NamedTempFile::with_suffix(".json").unwrap();
        array_writer.open(temp_file1.path()).await.unwrap();
        
        let series = Series::new(&*"TEST001", &*"Test".to_string());
        array_writer.write_series(&series).await.unwrap();
        array_writer.close().await.unwrap();
        
        let mut content1 = String::new();
        File::open(temp_file1.path()).unwrap().read_to_string(&mut content1).unwrap();
        assert!(content1.starts_with('['));
        assert!(content1.ends_with(']'));
        
        // Test line-delimited format
        let mut line_writer = JsonDataWriter::new(config);
        line_writer.set_array_format(false);
        
        let temp_file2 = NamedTempFile::with_suffix(".jsonl").unwrap();
        line_writer.open(temp_file2.path()).await.unwrap();
        line_writer.write_series(&series).await.unwrap();
        line_writer.close().await.unwrap();
        
        let mut content2 = String::new();
        File::open(temp_file2.path()).unwrap().read_to_string(&mut content2).unwrap();
        assert!(!content2.starts_with('['));
        assert!(!content2.ends_with(']'));
    }

    #[tokio::test]
    async fn test_supported_extensions() {
        let config = WriterConfig::default();
        let writer = JsonDataWriter::new(config);
        
        let extensions = writer.supported_extensions();
        assert!(extensions.contains(&"json".to_string()));
        assert!(extensions.contains(&"jsonl".to_string()));
    }

    #[tokio::test]
    async fn test_compression() {
        let mut config = WriterConfig::default();
        config.compress_output = true;
        config.compression_level = 6;
        
        let mut writer = JsonDataWriter::new(config);
        assert_eq!(writer.compression_level(), 6);
        
        writer.set_compression_level(3).unwrap();
        assert_eq!(writer.compression_level(), 3);
        
        // Test invalid compression level
        assert!(writer.set_compression_level(10).is_err());
    }
}