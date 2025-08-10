//! CSV writer implementation for BLS data processing
//!
//! This module provides a concrete implementation of the writer traits for
//! CSV format output. It supports buffered writing, compression, and various
//! CSV formatting options.

use async_trait::async_trait;
use csv::{Writer as CsvWriter, WriterBuilder};
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::data::model::{Lookup, Observation, Series, Survey};
use crate::data::writer::traits::{
    DataWriter, LookupWriter, ObservationWriter, SeriesWriter,
    SurveyWriter, WriteStats, WriterConfig,
};
use crate::error::types::{DataError, Result};
use crate::utils::validation::BLSValidationRules;

/// CSV writer implementation
pub struct CsvDataWriter {
    config: WriterConfig,
    stats: WriteStats,
    current_file: Option<PathBuf>,
    writer: Option<CsvWriter<Box<dyn Write + Send + Sync>>>,
    validation_rules: BLSValidationRules,
    headers_written: bool,
}

impl CsvDataWriter {
    /// Create a new CSV writer with the given configuration
    pub fn new(config: WriterConfig) -> Self {
        Self {
            config,
            stats: WriteStats::default(),
            current_file: None,
            writer: None,
            validation_rules: BLSValidationRules::default(),
            headers_written: false,
        }
    }

    /// Create a CSV writer with appropriate compression if configured
    fn create_writer(&self, file: File) -> Result<CsvWriter<Box<dyn Write + Send + Sync>>> {
        let writer: Box<dyn Write + Send + Sync> = if self.config.compress_output {
            let compression = Compression::new(self.config.compression_level as u32);
            Box::new(GzEncoder::new(BufWriter::new(file), compression))
        } else {
            Box::new(BufWriter::new(file))
        };

        let csv_writer = WriterBuilder::new()
            .delimiter(self.config.field_separator as u8)
            .has_headers(false) // We'll handle headers manually
            .buffer_capacity(self.config.buffer_size)
            .from_writer(writer);

        Ok(csv_writer)
    }

    /// Validate a record before writing
    fn validate_record<T>(&self, record: &T, record_type: &str) -> Result<bool>
    where
        T: std::fmt::Debug,
    {
        if !self.config.validate_on_write {
            return Ok(true);
        }

        // Basic validation - in a real implementation, this would be more sophisticated
        match record_type {
            "series" | "observation" | "lookup" | "survey" => Ok(true),
            _ => Err(DataError::ValidationError {
                message: format!("Unknown record type: {record_type}"),
                path: None,
                line: None,
            }
            .into()),
        }
    }

    /// Update statistics after writing records
    fn update_stats(
        &mut self,
        records_written: u64,
        bytes_written: u64,
        errors: u64,
        start_time: Instant,
    ) {
        self.stats.records_written += records_written;
        self.stats.bytes_written += bytes_written;
        self.stats.errors_encountered += errors;
        self.stats.write_time_ms += start_time.elapsed().as_millis() as u64;
    }

    /// Write headers for series data
    fn write_series_headers(&mut self) -> Result<()> {
        if !self.config.include_headers || self.headers_written {
            return Ok(());
        }

        if let Some(ref mut writer) = self.writer {
            let headers = vec![
                "series_id",
                "title",
                "area_code",
                "item_code",
                "frequency",
                "units",
                "seasonal_adjustment",
                "begin_year",
                "begin_period",
                "end_year",
                "end_period",
            ];

            writer
                .write_record(&headers)
                .map_err(|e| DataError::io_error(format!("Failed to write headers: {e}")))?;

            self.headers_written = true;
        }

        Ok(())
    }

    /// Write headers for observation data
    fn write_observation_headers(&mut self) -> Result<()> {
        if !self.config.include_headers || self.headers_written {
            return Ok(());
        }

        if let Some(ref mut writer) = self.writer {
            let headers = vec!["series_id", "year", "period", "value", "footnote_codes"];

            writer
                .write_record(&headers)
                .map_err(|e| DataError::io_error(format!("Failed to write headers: {e}")))?;

            self.headers_written = true;
        }

        Ok(())
    }

    /// Write headers for lookup data
    fn write_lookup_headers(&mut self) -> Result<()> {
        if !self.config.include_headers || self.headers_written {
            return Ok(());
        }

        if let Some(ref mut writer) = self.writer {
            // Flattened lookup entries: one row per entry across all tables
            let headers = vec!["table_id", "code", "description", "parent_code", "survey_code"];

            writer
                .write_record(&headers)
                .map_err(|e| DataError::io_error(format!("Failed to write headers: {e}")))?;

            self.headers_written = true;
        }

        Ok(())
    }

    /// Format a floating point value according to configuration
    fn format_float(&self, value: f64) -> String {
        format!(
            "{:.precision$}",
            value,
            precision = self.config.float_precision
        )
    }

    /// Convert a series to CSV record
    fn series_to_record(&self, series: &Series) -> Vec<String> {
        vec![
            series.series_id.to_string(),
            series.title().to_string(),
            series.area_code.to_string(),
            series.item_code.to_string(),
            format!("{:?}", series.frequency()),
            series
                .unit()
                .map(|u| format!("{u:?}"))
                .unwrap_or_default(),
            series.seasonal.to_string(),
            series.base_period.to_string(),
            series.periodicity_code.to_string(),
            series.base_code.to_string(),
            "".to_string(), // placeholder for end_period
        ]
    }

    /// Convert an observation to CSV record
    fn observation_to_record(&self, observation: &Observation) -> Vec<String> {
        vec![
            observation.series_id().to_string(),
            observation.year().to_string(),
            observation.period().to_string(),
            observation
                .numeric_value()
                .map(|v| self.format_float(v))
                .unwrap_or_else(|| "-".to_string()),
            "".to_string(), // placeholder for footnote_codes
        ]
    }

    /// Convert a lookup to CSV record
    fn lookup_to_record(&self, lookup: &Lookup) -> Vec<String> {
        vec![lookup.table_id.to_string(), lookup.table_name.to_string()]
    }
}

#[async_trait]
impl DataWriter for CsvDataWriter {
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
                Some("csv") | Some("tsv") | Some("txt") => Ok(true),
                Some("gz") => {
                    // Check if it's a compressed CSV
                    if let Some(stem) = path.file_stem() {
                        if let Some(stem_str) = stem.to_str() {
                            Ok(stem_str.ends_with(".csv") || stem_str.ends_with(".tsv"))
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
            return Err(DataError::unsupported_format(format!(
                "Cannot write CSV to: {}",
                path.display()
            ))
            .into());
        }

        // Check if file exists and we're not allowed to overwrite
        if path.exists() && !self.config.overwrite_existing {
            return Err(DataError::io_error(format!(
                "File already exists and overwrite is disabled: {}",
                path.display()
            ))
            .into());
        }

        let file = File::create(path)
            .map_err(|e| DataError::io_error(format!("Failed to create file: {e}")))?;

        let writer = self.create_writer(file)?;
        self.writer = Some(writer);
        self.current_file = Some(path.to_path_buf());
        self.headers_written = false;
        self.reset_stats();

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        if let Some(mut writer) = self.writer.take() {
            writer
                .flush()
                .map_err(|e| DataError::io_error(format!("Failed to flush writer: {e}")))?;
        }
        self.current_file = None;
        self.headers_written = false;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if let Some(ref mut writer) = self.writer {
            writer
                .flush()
                .map_err(|e| DataError::io_error(format!("Failed to flush writer: {e}")))?;
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
        vec!["csv".to_string(), "tsv".to_string(), "txt".to_string()]
    }
}

#[async_trait]
impl SeriesWriter for CsvDataWriter {
    async fn write_series(&mut self, series: &Series) -> Result<()> {
        let start_time = Instant::now();

        self.validate_record(series, "series")?;
        self.write_series_headers()?;

        let record = self.series_to_record(series);
        let record_size = record.iter().map(|s| s.len()).sum::<usize>() as u64;

        if let Some(ref mut writer) = self.writer {
            writer.write_record(&record).map_err(|e| {
                DataError::io_error(format!("Failed to write series record: {e}"))
            })?;

            self.update_stats(1, record_size, 0, start_time);
        } else {
            return Err(DataError::io_error("No writer available".to_string()).into());
        }

        Ok(())
    }

    async fn write_series_batch(&mut self, series: &[Series]) -> Result<()> {
        let start_time = Instant::now();
        let mut total_bytes = 0;
        let mut errors = 0;

        self.write_series_headers()?;

        // Pre-process all records to avoid borrow conflicts
        let mut processed_records = Vec::new();
        for s in series {
            match self.validate_record(s, "series") {
                Ok(_) => {
                    let record = self.series_to_record(s);
                    let record_size = record.iter().map(|s| s.len()).sum::<usize>() as u64;
                    total_bytes += record_size;
                    processed_records.push(record);
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }

        if let Some(ref mut writer) = self.writer {
            for record in processed_records {
                if let Err(e) = writer.write_record(&record) {
                    errors += 1;
                    eprintln!("Failed to write series record: {e}");
                }
            }
        } else {
            return Err(DataError::io_error("No writer available".to_string()).into());
        }

        self.update_stats(
            series.len() as u64 - errors,
            total_bytes,
            errors,
            start_time,
        );
        Ok(())
    }

    // Generic methods moved to SeriesWriterExt extension trait
}

#[async_trait]
impl ObservationWriter for CsvDataWriter {
    async fn write_observation(&mut self, observation: &Observation) -> Result<()> {
        let start_time = Instant::now();

        self.validate_record(observation, "observation")?;
        self.write_observation_headers()?;

        let record = self.observation_to_record(observation);
        let record_size = record.iter().map(|s| s.len()).sum::<usize>() as u64;

        if let Some(ref mut writer) = self.writer {
            writer.write_record(&record).map_err(|e| {
                DataError::io_error(format!("Failed to write observation record: {e}"))
            })?;

            self.update_stats(1, record_size, 0, start_time);
        } else {
            return Err(DataError::io_error("No writer available".to_string()).into());
        }

        Ok(())
    }

    async fn write_observations_batch(&mut self, observations: &[Observation]) -> Result<()> {
        let start_time = Instant::now();
        let mut total_bytes = 0;
        let mut errors = 0;

        self.write_observation_headers()?;

        // Pre-process all records to avoid borrow conflicts
        let mut processed_records = Vec::new();
        for obs in observations {
            match self.validate_record(obs, "observation") {
                Ok(_) => {
                    let record = self.observation_to_record(obs);
                    let record_size = record.iter().map(|s| s.len()).sum::<usize>() as u64;
                    total_bytes += record_size;
                    processed_records.push(record);
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }

        if let Some(ref mut writer) = self.writer {
            for record in processed_records {
                if let Err(e) = writer.write_record(&record) {
                    errors += 1;
                    eprintln!("Failed to write observation record: {e}");
                }
            }
        } else {
            return Err(DataError::io_error("No writer available".to_string()).into());
        }

        self.update_stats(
            observations.len() as u64 - errors,
            total_bytes,
            errors,
            start_time,
        );
        Ok(())
    }

    // Generic method write_all_observations moved to extension trait

    async fn write_observations_for_series(
        &mut self,
        series_id: &str,
        observations: &[Observation],
    ) -> Result<()> {
        // Filter observations for the specific series
        let filtered_observations: Vec<&Observation> = observations
            .iter()
            .filter(|obs| obs.series_id() == series_id)
            .collect();

        // Convert to owned observations for the batch write
        let owned_observations: Vec<Observation> =
            filtered_observations.into_iter().cloned().collect();

        self.write_observations_batch(&owned_observations).await
    }

    // Generic method write_observations_with_serializer moved to extension trait
}

#[async_trait]
impl LookupWriter for CsvDataWriter {
    async fn write_lookup(&mut self, lookup: &Lookup) -> Result<()> {
        let start_time = Instant::now();

        self.validate_record(lookup, "lookup")?;
        self.write_lookup_headers()?;

        let table_id = lookup.table_id.clone();
        let survey_code = lookup.survey_code.clone();

        let mut total_bytes = 0u64;
        let mut rows_written = 0u64;
        let mut errors = 0u64;

        if let Some(ref mut writer) = self.writer {
            for entry in lookup.entries.values() {
                let record = vec![
                    table_id.clone(),
                    entry.code.clone(),
                    entry.description.clone(),
                    entry.parent_code.clone().unwrap_or_default(),
                    survey_code.clone(),
                ];
                let record_size = record.iter().map(|s| s.len()).sum::<usize>() as u64;
                match writer.write_record(&record) {
                    Ok(_) => {
                        total_bytes += record_size;
                        rows_written += 1;
                    }
                    Err(e) => {
                        errors += 1;
                        eprintln!("Failed to write lookup record: {e}");
                    }
                }
            }
        } else {
            return Err(DataError::io_error("No writer available".to_string()).into());
        }

        self.update_stats(rows_written, total_bytes, errors, start_time);
        Ok(())
    }

    async fn write_lookups_batch(&mut self, lookups: &[Lookup]) -> Result<()> {
        let start_time = Instant::now();
        let mut total_bytes = 0u64;
        let mut errors = 0u64;
        let mut rows_written = 0u64;

        self.write_lookup_headers()?;

        // Preprocess records to avoid borrow conflicts of self.writer
        let mut records: Vec<Vec<String>> = Vec::new();
        for lookup in lookups {
            self.validate_record(lookup, "lookup")?;
            let table_id = lookup.table_id.clone();
            let survey_code = lookup.survey_code.clone();
            for entry in lookup.entries.values() {
                let record = vec![
                    table_id.clone(),
                    entry.code.clone(),
                    entry.description.clone(),
                    entry.parent_code.clone().unwrap_or_default(),
                    survey_code.clone(),
                ];
                let record_size = record.iter().map(|s| s.len()).sum::<usize>() as u64;
                total_bytes += record_size;
                records.push(record);
            }
        }

        if let Some(ref mut writer) = self.writer {
            for record in records {
                if let Err(e) = writer.write_record(&record) {
                    errors += 1;
                    eprintln!("Failed to write lookup record: {e}");
                } else {
                    rows_written += 1;
                }
            }
        } else {
            return Err(DataError::io_error("No writer available".to_string()).into());
        }

        self.update_stats(rows_written, total_bytes, errors, start_time);
        Ok(())
    }

    // Generic method write_all_lookups moved to extension trait

    // Generic method write_lookups_with_serializer moved to extension trait
}

#[async_trait]
impl SurveyWriter for CsvDataWriter {
    async fn write_survey(&mut self, survey: &Survey) -> Result<()> {
        let start_time = Instant::now();

        self.validate_record(survey, "survey")?;

        if let Some(ref mut writer) = self.writer {
            // For survey metadata, we'll write key-value pairs
            let records = vec![
                vec!["survey_code".to_string(), survey.survey_code.to_string()],
                vec!["survey_name".to_string(), survey.name.to_string()],
                vec!["description".to_string(), "".to_string()], // placeholder
            ];

            let mut total_bytes = 0;
            for record in records {
                let record_size = record.iter().map(|s| s.len()).sum::<usize>() as u64;
                total_bytes += record_size;

                writer.write_record(&record).map_err(|e| {
                    DataError::io_error(format!("Failed to write survey record: {e}"))
                })?;
            }

            self.update_stats(1, total_bytes, 0, start_time);
        } else {
            return Err(DataError::io_error("No writer available".to_string()).into());
        }

        Ok(())
    }

    // Generic method write_survey_with_format moved to extension trait
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_csv_writer_creation() {
        let config = WriterConfig::default();
        let writer = CsvDataWriter::new(config);
        assert!(!writer.is_open());
        assert!(writer.current_file().is_none());
    }

    #[tokio::test]
    async fn test_can_write_csv() {
        let config = WriterConfig::default();
        let writer = CsvDataWriter::new(config);

        let csv_path = Path::new("test.csv");
        assert!(writer.can_write(csv_path).unwrap());

        let txt_path = Path::new("test.txt");
        assert!(writer.can_write(txt_path).unwrap());

        let invalid_path = Path::new("test.json");
        assert!(!writer.can_write(invalid_path).unwrap());
    }

    #[tokio::test]
    async fn test_write_series() {
        let config = WriterConfig::default();
        let mut writer = CsvDataWriter::new(config);

        let temp_file = NamedTempFile::with_suffix(".csv").unwrap();
        writer.open(temp_file.path()).await.unwrap();

        let series = Series::new(&*"TEST001".to_string(), &*"Test Series".to_string());

        writer.write_series(&series).await.unwrap();
        writer.close().await.unwrap();

        // Verify the file was written
        let mut file_content = String::new();
        let mut file = File::open(temp_file.path()).unwrap();
        file.read_to_string(&mut file_content).unwrap();

        assert!(file_content.contains("TEST001"));
        assert!(file_content.contains("Test Series"));
    }

    #[tokio::test]
    async fn test_supported_extensions() {
        let config = WriterConfig::default();
        let writer = CsvDataWriter::new(config);

        let extensions = writer.supported_extensions();
        assert!(extensions.contains(&"csv".to_string()));
        assert!(extensions.contains(&"tsv".to_string()));
        assert!(extensions.contains(&"txt".to_string()));
    }
}
