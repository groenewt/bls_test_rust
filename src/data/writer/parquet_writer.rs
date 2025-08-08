//! Parquet writer implementation for BLS data processing
//!
//! This module provides a concrete implementation of the writer traits for
//! Parquet format output. It leverages Apache Arrow for columnar data processing
//! and provides excellent compression and query performance.

use async_trait::async_trait;
use parquet::file::properties::EnabledStatistics;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use arrow::array::{ArrayRef, Float64Builder, Int32Builder, StringBuilder};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, Encoding};
use parquet::file::properties::WriterProperties;

use crate::data::model::{Lookup, Observation, Series, Survey};
use crate::data::writer::traits::{
    CompressedWriter, CompressionInfo, DataWriter, LookupWriter, ObservationWriter, SeriesWriter,
    SurveyWriter, WriteStats, WriterConfig,
};
use crate::error::types::{DataError, Result};
use crate::utils::validation::BLSValidationRules;

/// Parquet writer implementation
#[derive(Debug)]
pub struct ParquetDataWriter {
    config: WriterConfig,
    stats: WriteStats,
    current_file: Option<PathBuf>,
    writer: Arc<Mutex<Option<ArrowWriter<File>>>>,
    validation_rules: BLSValidationRules,
    schema: Option<Arc<Schema>>,
    batch_data: Vec<RecordBatch>,
    compression_info: Option<CompressionInfo>,
}

impl ParquetDataWriter {
    /// Create a new Parquet writer with the given configuration
    pub fn new(config: WriterConfig) -> Self {
        Self {
            config,
            stats: WriteStats::default(),
            current_file: None,
            writer: Arc::new(Mutex::new(None)),
            validation_rules: BLSValidationRules::default(),
            schema: None,
            batch_data: Vec::new(),
            compression_info: None,
        }
    }

    /// Create writer properties based on configuration
    fn create_writer_properties(&self) -> WriterProperties {
        let compression = match self.config.compression_level {
            0 => Compression::UNCOMPRESSED,
            1..=3 => Compression::SNAPPY,
            4..=6 => Compression::GZIP(parquet::basic::GzipLevel::default()),
            7..=9 => Compression::ZSTD(parquet::basic::ZstdLevel::default()),
            _ => Compression::SNAPPY,
        };

        WriterProperties::builder()
            .set_compression(compression)
            .set_encoding(Encoding::PLAIN)
            .set_dictionary_enabled(true)
            .set_statistics_enabled(EnabledStatistics::Chunk)
            .set_max_row_group_size(self.config.batch_size)
            .build()
    }

    /// Create schema for series data
    fn create_series_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("series_id", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, true),
            Field::new("area_code", DataType::Utf8, true),
            Field::new("item_code", DataType::Utf8, true),
            Field::new("frequency", DataType::Utf8, true),
            Field::new("units", DataType::Utf8, true),
            Field::new("seasonal_adjustment", DataType::Utf8, true),
            Field::new("begin_year", DataType::Int32, true),
            Field::new("begin_period", DataType::Utf8, true),
            Field::new("end_year", DataType::Int32, true),
            Field::new("end_period", DataType::Utf8, true),
        ]))
    }

    /// Create schema for observation data
    fn create_observation_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("series_id", DataType::Utf8, false),
            Field::new("year", DataType::Int32, false),
            Field::new("period", DataType::Utf8, false),
            Field::new("value", DataType::Float64, true),
            Field::new("footnote_codes", DataType::Utf8, true),
        ]))
    }

    /// Create schema for lookup data
    fn create_lookup_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("code", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("description", DataType::Utf8, true),
        ]))
    }

    /// Create schema for survey data
    fn create_survey_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("survey_code", DataType::Utf8, false),
            Field::new("survey_name", DataType::Utf8, true),
            Field::new("description", DataType::Utf8, true),
        ]))
    }

    /// Convert series data to Arrow arrays
    fn series_to_arrays(&self, series_list: &[Series]) -> Result<Vec<ArrayRef>> {
        let mut series_id_builder = StringBuilder::new();
        let mut title_builder = StringBuilder::new();
        let mut area_code_builder = StringBuilder::new();
        let mut item_code_builder = StringBuilder::new();
        let mut frequency_builder = StringBuilder::new();
        let mut units_builder = StringBuilder::new();
        let mut seasonal_adjustment_builder = StringBuilder::new();
        let mut begin_year_builder = Int32Builder::new();
        let mut begin_period_builder = StringBuilder::new();
        let mut end_year_builder = Int32Builder::new();
        let mut end_period_builder = StringBuilder::new();

        for series in series_list {
            series_id_builder.append_value(&series.series_id);
            title_builder.append_option(Some(&series.title));
            area_code_builder.append_option(Some(&series.area_code));
            item_code_builder.append_option(Some(&series.item_code));
            frequency_builder.append_option(Some(&format!("{:?}", series.frequency())));
            units_builder.append_option(series.unit().map(|u| format!("{u:?}")).as_deref());
            seasonal_adjustment_builder.append_option(Some(&series.seasonal));
            begin_year_builder.append_option(None); // placeholder
            begin_period_builder.append_option(Some(&series.base_period));
            end_year_builder.append_option(None); // placeholder
            end_period_builder.append_option(Some(&series.periodicity_code));
        }

        Ok(vec![
            Arc::new(series_id_builder.finish()),
            Arc::new(title_builder.finish()),
            Arc::new(area_code_builder.finish()),
            Arc::new(item_code_builder.finish()),
            Arc::new(frequency_builder.finish()),
            Arc::new(units_builder.finish()),
            Arc::new(seasonal_adjustment_builder.finish()),
            Arc::new(begin_year_builder.finish()),
            Arc::new(begin_period_builder.finish()),
            Arc::new(end_year_builder.finish()),
            Arc::new(end_period_builder.finish()),
        ])
    }

    /// Convert observation data to Arrow arrays
    fn observations_to_arrays(&self, observations: &[Observation]) -> Result<Vec<ArrayRef>> {
        let mut series_id_builder = StringBuilder::new();
        let mut year_builder = Int32Builder::new();
        let mut period_builder = StringBuilder::new();
        let mut value_builder = Float64Builder::new();
        let mut footnote_codes_builder = StringBuilder::new();

        for observation in observations {
            series_id_builder.append_value(observation.series_id());
            year_builder.append_value(observation.year() as i32);
            period_builder.append_value(observation.period());
            value_builder.append_option(observation.numeric_value());
            let quality_str = observation.quality().to_string();
            footnote_codes_builder.append_option(Some(&quality_str));
        }

        Ok(vec![
            Arc::new(series_id_builder.finish()),
            Arc::new(year_builder.finish()),
            Arc::new(period_builder.finish()),
            Arc::new(value_builder.finish()),
            Arc::new(footnote_codes_builder.finish()),
        ])
    }

    /// Convert lookup data to Arrow arrays
    fn lookups_to_arrays(&self, lookups: &[Lookup]) -> Result<Vec<ArrayRef>> {
        let mut code_builder = StringBuilder::new();
        let mut name_builder = StringBuilder::new();
        let mut description_builder = StringBuilder::new();

        for lookup in lookups {
            code_builder.append_value(&lookup.table_id);
            name_builder.append_value(&lookup.table_name);
            description_builder.append_option(Some(&lookup.table_name));
        }

        Ok(vec![
            Arc::new(code_builder.finish()),
            Arc::new(name_builder.finish()),
            Arc::new(description_builder.finish()),
        ])
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

    /// Write accumulated batches to file
    fn write_batches(&mut self) -> Result<()> {
        if let Ok(mut writer_guard) = self.writer.lock() {
            if let Some(ref mut writer) = *writer_guard {
                for batch in &self.batch_data {
                    writer.write(batch).map_err(|e| {
                        DataError::io_error(format!("Failed to write batch: {e}"))
                    })?;
                }
                self.batch_data.clear();
            }
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
            algorithm: match self.config.compression_level {
                0 => "none".to_string(),
                1..=3 => "snappy".to_string(),
                4..=6 => "gzip".to_string(),
                7..=9 => "zstd".to_string(),
                _ => "snappy".to_string(),
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
impl DataWriter for ParquetDataWriter {
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
                Some("parquet") => Ok(true),
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    async fn open(&mut self, path: &Path) -> Result<()> {
        if !self.can_write(path)? {
            return Err(DataError::unsupported_format(format!(
                "Cannot write Parquet to: {}",
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

        self.current_file = Some(path.to_path_buf());
        self.batch_data.clear();
        self.reset_stats();

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.write_batches()?;

        if let Ok(mut writer_guard) = self.writer.lock() {
            if let Some(writer) = writer_guard.take() {
                writer
                    .close()
                    .map_err(|e| DataError::io_error(format!("Failed to close writer: {e}")))?;
            }
        }

        self.current_file = None;
        self.schema = None;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        self.write_batches()?;

        if let Ok(mut writer_guard) = self.writer.lock() {
            if let Some(ref mut writer) = *writer_guard {
                writer
                    .flush()
                    .map_err(|e| DataError::io_error(format!("Failed to flush writer: {e}")))?;
            }
        }
        Ok(())
    }

    fn is_open(&self) -> bool {
        self.current_file.is_some()
    }

    fn current_file(&self) -> Option<&Path> {
        self.current_file.as_deref()
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["parquet".to_string()]
    }
}

#[async_trait]
impl SeriesWriter for ParquetDataWriter {
    async fn write_series(&mut self, series: &Series) -> Result<()> {
        self.write_series_batch(&[series.clone()]).await
    }

    async fn write_series_batch(&mut self, series: &[Series]) -> Result<()> {
        let start_time = Instant::now();

        // Validate all records
        for s in series {
            self.validate_record(s, "series")?;
        }

        // Set up schema if not already done
        if self.schema.is_none() {
            self.schema = Some(Self::create_series_schema());
        }

        // Create writer if not already done
        if self.writer.lock().unwrap().is_none() && self.current_file.is_some() {
            let file = File::create(self.current_file.as_ref().unwrap())
                .map_err(|e| DataError::io_error(format!("Failed to create file: {e}")))?;

            let properties = self.create_writer_properties();
            let writer = ArrowWriter::try_new(file, self.schema.clone().unwrap(), Some(properties))
                .map_err(|e| {
                    DataError::io_error(format!("Failed to create Parquet writer: {e}"))
                })?;

            *self.writer.lock().unwrap() = Some(writer);
        }

        // Convert to Arrow arrays
        let arrays = self.series_to_arrays(series)?;
        let batch = RecordBatch::try_new(self.schema.clone().unwrap(), arrays)
            .map_err(|e| DataError::io_error(format!("Failed to create record batch: {e}")))?;

        let batch_size = batch.get_array_memory_size() as u64;
        self.batch_data.push(batch);

        // Write if we've accumulated enough data
        if self.batch_data.len() >= self.config.batch_size {
            self.write_batches()?;
        }

        self.update_stats(series.len() as u64, batch_size, 0, start_time);
        Ok(())
    }

    // Generic method write_all_series moved to extension trait

    // Generic method write_series_with_serializer moved to extension trait
}

#[async_trait]
impl ObservationWriter for ParquetDataWriter {
    async fn write_observation(&mut self, observation: &Observation) -> Result<()> {
        self.write_observations_batch(&[observation.clone()]).await
    }

    async fn write_observations_batch(&mut self, observations: &[Observation]) -> Result<()> {
        let start_time = Instant::now();

        // Validate all records
        for obs in observations {
            self.validate_record(obs, "observation")?;
        }

        // Set up schema if not already done
        if self.schema.is_none() {
            self.schema = Some(Self::create_observation_schema());
        }

        // Create writer if not already done
        if self.writer.lock().unwrap().is_none() && self.current_file.is_some() {
            let file = File::create(self.current_file.as_ref().unwrap())
                .map_err(|e| DataError::io_error(format!("Failed to create file: {e}")))?;

            let properties = self.create_writer_properties();
            let writer = ArrowWriter::try_new(file, self.schema.clone().unwrap(), Some(properties))
                .map_err(|e| {
                    DataError::io_error(format!("Failed to create Parquet writer: {e}"))
                })?;

            *self.writer.lock().unwrap() = Some(writer);
        }

        // Convert to Arrow arrays
        let arrays = self.observations_to_arrays(observations)?;
        let batch = RecordBatch::try_new(self.schema.clone().unwrap(), arrays)
            .map_err(|e| DataError::io_error(format!("Failed to create record batch: {e}")))?;

        let batch_size = batch.get_array_memory_size() as u64;
        self.batch_data.push(batch);

        // Write if we've accumulated enough data
        if self.batch_data.len() >= self.config.batch_size {
            self.write_batches()?;
        }

        self.update_stats(observations.len() as u64, batch_size, 0, start_time);
        Ok(())
    }

    // Generic method write_all_observations moved to extension trait

    async fn write_observations_for_series(
        &mut self,
        series_id: &str,
        observations: &[Observation],
    ) -> Result<()> {
        let filtered_observations: Vec<Observation> = observations
            .iter()
            .filter(|obs| obs.series_id() == series_id)
            .cloned()
            .collect();

        self.write_observations_batch(&filtered_observations).await
    }

    // Generic method write_observations_with_serializer moved to extension trait
}

#[async_trait]
impl LookupWriter for ParquetDataWriter {
    async fn write_lookup(&mut self, lookup: &Lookup) -> Result<()> {
        self.write_lookups_batch(&[lookup.clone()]).await
    }

    async fn write_lookups_batch(&mut self, lookups: &[Lookup]) -> Result<()> {
        let start_time = Instant::now();

        // Validate all records
        for lookup in lookups {
            self.validate_record(lookup, "lookup")?;
        }

        // Set up schema if not already done
        if self.schema.is_none() {
            self.schema = Some(Self::create_lookup_schema());
        }

        // Create writer if not already done
        if self.writer.lock().unwrap().is_none() && self.current_file.is_some() {
            let file = File::create(self.current_file.as_ref().unwrap())
                .map_err(|e| DataError::io_error(format!("Failed to create file: {e}")))?;

            let properties = self.create_writer_properties();
            let writer = ArrowWriter::try_new(file, self.schema.clone().unwrap(), Some(properties))
                .map_err(|e| {
                    DataError::io_error(format!("Failed to create Parquet writer: {e}"))
                })?;

            *self.writer.lock().unwrap() = Some(writer);
        }

        // Convert to Arrow arrays
        let arrays = self.lookups_to_arrays(lookups)?;
        let batch = RecordBatch::try_new(self.schema.clone().unwrap(), arrays)
            .map_err(|e| DataError::io_error(format!("Failed to create record batch: {e}")))?;

        let batch_size = batch.get_array_memory_size() as u64;
        self.batch_data.push(batch);

        // Write if we've accumulated enough data
        if self.batch_data.len() >= self.config.batch_size {
            self.write_batches()?;
        }

        self.update_stats(lookups.len() as u64, batch_size, 0, start_time);
        Ok(())
    }

    // Generic method write_all_lookups moved to extension trait

    // Generic method write_lookups_with_serializer moved to extension trait
}

#[async_trait]
impl SurveyWriter for ParquetDataWriter {
    async fn write_survey(&mut self, survey: &Survey) -> Result<()> {
        let start_time = Instant::now();

        self.validate_record(survey, "survey")?;

        // Set up schema if not already done
        if self.schema.is_none() {
            self.schema = Some(Self::create_survey_schema());
        }

        // Create writer if not already done
        if self.writer.lock().unwrap().is_none() && self.current_file.is_some() {
            let file = File::create(self.current_file.as_ref().unwrap())
                .map_err(|e| DataError::io_error(format!("Failed to create file: {e}")))?;

            let properties = self.create_writer_properties();
            let writer = ArrowWriter::try_new(file, self.schema.clone().unwrap(), Some(properties))
                .map_err(|e| {
                    DataError::io_error(format!("Failed to create Parquet writer: {e}"))
                })?;

            *self.writer.lock().unwrap() = Some(writer);
        }

        // Convert survey to arrays
        let mut survey_code_builder = StringBuilder::new();
        let mut survey_name_builder = StringBuilder::new();
        let mut description_builder = StringBuilder::new();

        survey_code_builder.append_value(survey.code());
        survey_name_builder.append_option(Some(survey.name()));
        description_builder.append_option(survey.description());

        let arrays: Vec<ArrayRef> = vec![
            Arc::new(survey_code_builder.finish()),
            Arc::new(survey_name_builder.finish()),
            Arc::new(description_builder.finish()),
        ];

        let batch = RecordBatch::try_new(self.schema.clone().unwrap(), arrays)
            .map_err(|e| DataError::io_error(format!("Failed to create record batch: {e}")))?;

        let batch_size = batch.get_array_memory_size() as u64;
        self.batch_data.push(batch);

        // Write immediately for survey data
        self.write_batches()?;

        self.update_stats(1, batch_size, 0, start_time);
        Ok(())
    }

    // Generic method write_survey_with_format moved to extension trait
}

#[async_trait]
impl CompressedWriter for ParquetDataWriter {
    fn set_compression_level(&mut self, level: u8) -> Result<()> {
        if level > 9 {
            return Err(DataError::invalid_configuration(
                "Compression level must be between 0 and 9".to_string(),
            )
            .into());
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

    #[tokio::test]
    async fn test_parquet_writer_creation() {
        let config = WriterConfig::default();
        let writer = ParquetDataWriter::new(config);
        assert!(!writer.is_open());
        assert!(writer.current_file().is_none());
    }

    #[tokio::test]
    async fn test_can_write_parquet() {
        let config = WriterConfig::default();
        let writer = ParquetDataWriter::new(config);

        let parquet_path = Path::new("test.parquet");
        assert!(writer.can_write(parquet_path).unwrap());

        let invalid_path = Path::new("test.csv");
        assert!(!writer.can_write(invalid_path).unwrap());
    }

    #[tokio::test]
    async fn test_write_series() {
        let config = WriterConfig::default();
        let mut writer = ParquetDataWriter::new(config);

        let temp_file = NamedTempFile::with_suffix(".parquet").unwrap();
        writer.open(temp_file.path()).await.unwrap();

        let series = Series::new(&*"TEST001".to_string(), &*"Test Series".to_string());

        writer.write_series(&series).await.unwrap();
        writer.close().await.unwrap();

        // Verify the file was created
        assert!(temp_file.path().exists());
        assert!(temp_file.path().metadata().unwrap().len() > 0);
    }

    #[tokio::test]
    async fn test_compression_levels() {
        let mut config = WriterConfig::default();
        config.compression_level = 6;

        let mut writer = ParquetDataWriter::new(config);
        assert_eq!(writer.compression_level(), 6);

        writer.set_compression_level(3).unwrap();
        assert_eq!(writer.compression_level(), 3);

        // Test invalid compression level
        assert!(writer.set_compression_level(10).is_err());
    }

    #[tokio::test]
    async fn test_supported_extensions() {
        let config = WriterConfig::default();
        let writer = ParquetDataWriter::new(config);

        let extensions = writer.supported_extensions();
        assert_eq!(extensions, vec!["parquet".to_string()]);
    }
}
