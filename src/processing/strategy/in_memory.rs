//! In-memory processing strategy for BLS data processing
//!
//! This module provides an in-memory processing strategy that loads all data
//! into memory for fast processing. It's optimized for small to medium-sized
//! datasets that can fit comfortably in available RAM.

use async_trait::async_trait;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use crate::data::model::{Lookup, Observation, Series, Survey};
use crate::data::reader::{
    DataReader, LookupReader, ObservationReader, SeriesReader, SurveyReader,
    create_optimized_reader,
};
use crate::data::writer::{
    DataWriter, LookupWriter, ObservationWriter, SeriesWriter, SurveyWriter,
    create_optimized_writer,
};
use crate::error::types::{ProcessingError, Result};
use crate::processing::traits::{
    DataProcessor, ProcessingConfig, ProcessingContext, ProcessingInput, ProcessingOutput,
    ProcessingStats, ProcessingStrategy,
};
use crate::utils::validation::BLSValidationRules;

/// In-memory processing strategy implementation
pub struct InMemoryProcessor {
    config: ProcessingConfig,
    stats: ProcessingStats,
    validation_rules: BLSValidationRules,
}

impl InMemoryProcessor {
    /// Create a new in-memory processor with the given configuration
    pub fn new(config: ProcessingConfig) -> Self {
        Self {
            config,
            stats: ProcessingStats::default(),
            validation_rules: BLSValidationRules::default(),
        }
    }

    /// Load all data from input sources into memory
    async fn load_all_data(&mut self, input: &ProcessingInput) -> Result<InMemoryDataSet> {
        let start_time = Instant::now();
        let mut dataset = InMemoryDataSet::new();
        let mut total_bytes = 0;
        let mut total_records = 0;

        for path in &input.paths {
            let path_obj = Path::new(path);
            let mut reader = create_optimized_reader(path_obj)?;

            reader.open(path_obj).await?;

            // Determine data type based on file name or format hint
            let data_type = self.determine_data_type(path, &input.format_hint)?;

            match data_type.as_str() {
                "series" => {
                    // Try to downcast to concrete types that implement SeriesReader
                    if let Some(file_reader) = reader
                        .as_any()
                        .downcast_ref::<crate::data::reader::file_reader::FileReader>(
                    ) {
                        let series_data = self.load_series_data(file_reader).await?;
                        total_records += series_data.len() as u64;
                        dataset.series.extend(series_data);
                    } else if let Some(mmap_reader) = reader
                        .as_any()
                        .downcast_ref::<crate::data::reader::mmap_reader::MmapReader>(
                    ) {
                        let series_data = self.load_series_data(mmap_reader).await?;
                        total_records += series_data.len() as u64;
                        dataset.series.extend(series_data);
                    }
                }
                "observations" => {
                    // Try to downcast to concrete types that implement ObservationReader
                    if let Some(file_reader) = reader
                        .as_any()
                        .downcast_ref::<crate::data::reader::file_reader::FileReader>(
                    ) {
                        let obs_data = self.load_observation_data(file_reader).await?;
                        total_records += obs_data.len() as u64;
                        dataset.observations.extend(obs_data);
                    }
                }
                "lookups" => {
                    // Try to downcast to concrete types that implement LookupReader
                    if let Some(file_reader) = reader
                        .as_any()
                        .downcast_ref::<crate::data::reader::file_reader::FileReader>(
                    ) {
                        let lookup_data = self.load_lookup_data(file_reader).await?;
                        total_records += lookup_data.len() as u64;
                        dataset.lookups.extend(lookup_data);
                    }
                }
                "survey" => {
                    // Survey loading not yet implemented
                    return Err(ProcessingError::NotImplemented(
                        "Survey data loading not yet implemented".to_string(),
                    )
                    .into());
                }
                _ => {
                    return Err(ProcessingError::UnsupportedDataType(format!(
                        "Unknown data type: {data_type}"
                    ))
                    .into());
                }
            }

            total_bytes += reader.stats().bytes_processed;
            reader.close().await?;
        }

        self.stats.records_processed += total_records;
        self.stats.bytes_processed += total_bytes;
        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;

        Ok(dataset)
    }

    /// Load series data from a reader
    async fn load_series_data(&self, _reader: &dyn SeriesReader) -> Result<Vec<Series>> {
        // For now, return empty data - proper implementation will load from reader
        // Focus on getting config integration working first per config_integration_steps.md
        Ok(Vec::new())
    }

    /// Load observation data from a reader
    async fn load_observation_data(
        &self,
        _reader: &dyn ObservationReader,
    ) -> Result<Vec<Observation>> {
        // For now, return empty data - proper implementation will load from reader
        Ok(Vec::new())
    }

    /// Load lookup data from a reader
    async fn load_lookup_data(&self, _reader: &dyn LookupReader) -> Result<Vec<Lookup>> {
        // For now, return empty data - proper implementation will load from reader
        Ok(Vec::new())
    }

    /// Load survey data from a reader
    async fn load_survey_data(&self, _reader: &dyn SurveyReader) -> Result<Survey> {
        // For now, return error - will be replaced with YAML config loading per config_integration_steps.md
        Err(ProcessingError::NotImplemented("Survey data loading will be replaced with YAML config integration".to_string()).into())
    }

    /// Determine data type from file path and format hint
    fn determine_data_type(&self, path: &str, format_hint: &Option<String>) -> Result<String> {
        if let Some(hint) = format_hint {
            return Ok(hint.clone());
        }

        let path_obj = Path::new(path);
        if let Some(filename) = path_obj.file_name().and_then(|n| n.to_str()) {
            if filename.contains(".series") {
                Ok("series".to_string())
            } else if filename.contains(".data") {
                Ok("observations".to_string())
            } else if filename.contains(".area") || filename.contains(".item") {
                Ok("lookups".to_string())
            } else {
                Ok("series".to_string()) // Default
            }
        } else {
            Ok("series".to_string()) // Default
        }
    }

    /// Process data in parallel using Rayon
    fn process_data_parallel(&mut self, dataset: &mut InMemoryDataSet) -> Result<()> {
        let start_time = Instant::now();

        // Process series data in parallel
        if !dataset.series.is_empty() {
            dataset.series.par_iter_mut().for_each(|series| {
                // Apply transformations, validations, etc.
                self.process_series(series);
            });
        }

        // Process observations data in parallel
        if !dataset.observations.is_empty() {
            dataset.observations.par_iter_mut().for_each(|observation| {
                // Apply transformations, validations, etc.
                self.process_observation(observation);
            });
        }

        // Process lookups data in parallel
        if !dataset.lookups.is_empty() {
            dataset.lookups.par_iter_mut().for_each(|lookup| {
                // Apply transformations, validations, etc.
                self.process_lookup(lookup);
            });
        }

        // Process surveys (typically small, so no need for parallelization)
        for survey in &mut dataset.surveys {
            self.process_survey(survey)?;
        }

        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;
        self.stats.threads_used = rayon::current_num_threads();

        Ok(())
    }

    /// Process a single series record
    fn process_series(&self, series: &mut Series) {
        // Apply validation
        if self.config.validate_data
            && self
                .validation_rules
                .validate_series_record(&[series.id().to_string(), series.title().to_string()]).is_err()
            {
                // Handle validation error
            }

        // Apply transformations (placeholder)
        // In a real implementation, this would apply configured transformations
    }

    /// Process a single observation record
    fn process_observation(&self, observation: &mut Observation) {
        // Apply validation
        if self.config.validate_data
            && self.validation_rules.validate_observation_record(&[
                observation.series_id().to_string(),
                observation.year().to_string(),
                observation.period().to_string(),
            ]).is_err() {
                // Handle validation error
            }

        // Apply transformations (placeholder)
    }

    /// Process a single lookup record
    fn process_lookup(&self, lookup: &mut Lookup) {
        // Apply validation
        if self.config.validate_data
            && self
                .validation_rules
                .validate_lookup_record(&[lookup.table_id.clone(), lookup.table_name.clone()]).is_err()
            {
                // Handle validation error
            }

        // Apply transformations (placeholder)
    }

    /// Process a single survey record
    fn process_survey(&self, survey: &mut Survey) -> Result<()> {
        // Apply validation and transformations
        Ok(())
    }

    /// Write processed data to output destinations
    async fn write_processed_data(
        &mut self,
        dataset: &InMemoryDataSet,
        output: &ProcessingOutput,
    ) -> Result<()> {
        let start_time = Instant::now();

        for (i, path) in output.paths.iter().enumerate() {
            let path_obj = Path::new(path);
            let mut writer = create_optimized_writer(path_obj)?;

            writer.open(path_obj).await?;

            // Write data based on what's available in the dataset
            if !dataset.series.is_empty() {
                // Try to downcast to concrete types that implement SeriesWriter
                if let Some(csv_writer) = writer
                    .as_any()
                    .downcast_ref::<crate::data::writer::csv_writer::CsvDataWriter>(
                ) {
                    self.write_series_data(csv_writer, &dataset.series).await?;
                } else if let Some(json_writer) = writer
                    .as_any()
                    .downcast_ref::<crate::data::writer::json_writer::JsonDataWriter>(
                ) {
                    self.write_series_data(json_writer, &dataset.series).await?;
                } else if let Some(parquet_writer) =
                    writer
                        .as_any()
                        .downcast_ref::<crate::data::writer::parquet_writer::ParquetDataWriter>()
                {
                    self.write_series_data(parquet_writer, &dataset.series)
                        .await?;
                }
            }

            if !dataset.observations.is_empty() {
                // Try to downcast to concrete types that implement ObservationWriter
                if let Some(csv_writer) = writer
                    .as_any()
                    .downcast_ref::<crate::data::writer::csv_writer::CsvDataWriter>(
                ) {
                    self.write_observation_data(csv_writer, &dataset.observations)
                        .await?;
                } else if let Some(json_writer) = writer
                    .as_any()
                    .downcast_ref::<crate::data::writer::json_writer::JsonDataWriter>(
                ) {
                    self.write_observation_data(json_writer, &dataset.observations)
                        .await?;
                } else if let Some(parquet_writer) =
                    writer
                        .as_any()
                        .downcast_ref::<crate::data::writer::parquet_writer::ParquetDataWriter>()
                {
                    self.write_observation_data(parquet_writer, &dataset.observations)
                        .await?;
                }
            }

            if !dataset.lookups.is_empty() {
                // Try to downcast to concrete types that implement LookupWriter
                if let Some(csv_writer) = writer
                    .as_any()
                    .downcast_ref::<crate::data::writer::csv_writer::CsvDataWriter>(
                ) {
                    self.write_lookup_data(csv_writer, &dataset.lookups).await?;
                } else if let Some(json_writer) = writer
                    .as_any()
                    .downcast_ref::<crate::data::writer::json_writer::JsonDataWriter>(
                ) {
                    self.write_lookup_data(json_writer, &dataset.lookups)
                        .await?;
                } else if let Some(parquet_writer) =
                    writer
                        .as_any()
                        .downcast_ref::<crate::data::writer::parquet_writer::ParquetDataWriter>()
                {
                    self.write_lookup_data(parquet_writer, &dataset.lookups)
                        .await?;
                }
            }

            if !dataset.surveys.is_empty() {
                // Try to downcast to concrete types that implement SurveyWriter
                for survey in &dataset.surveys {
                    if let Some(csv_writer) = writer
                        .as_any()
                        .downcast_ref::<crate::data::writer::csv_writer::CsvDataWriter>(
                    ) {
                        self.write_survey_data(csv_writer, survey).await?;
                    } else if let Some(json_writer) =
                        writer
                            .as_any()
                            .downcast_ref::<crate::data::writer::json_writer::JsonDataWriter>()
                    {
                        self.write_survey_data(json_writer, survey).await?;
                    } else if let Some(parquet_writer) =
                        writer
                            .as_any()
                            .downcast_ref::<crate::data::writer::parquet_writer::ParquetDataWriter>(
                            )
                    {
                        self.write_survey_data(parquet_writer, survey).await?;
                    }
                }
            }

            writer.close().await?;
        }

        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;
        Ok(())
    }

    /// Write series data using a writer
    async fn write_series_data(&self, writer: &dyn SeriesWriter, series: &[Series]) -> Result<()> {
        // This would be properly implemented with async trait handling
        Ok(())
    }

    /// Write observation data using a writer
    async fn write_observation_data(
        &self,
        writer: &dyn ObservationWriter,
        observations: &[Observation],
    ) -> Result<()> {
        // This would be properly implemented with async trait handling
        Ok(())
    }

    /// Write lookup data using a writer
    async fn write_lookup_data(&self, writer: &dyn LookupWriter, lookups: &[Lookup]) -> Result<()> {
        // This would be properly implemented with async trait handling
        Ok(())
    }

    /// Write survey data using a writer
    async fn write_survey_data(&self, writer: &dyn SurveyWriter, survey: &Survey) -> Result<()> {
        // This would be properly implemented with async trait handling
        Ok(())
    }

    /// Estimate memory usage for the given input
    fn estimate_memory_usage_internal(&self, input: &ProcessingInput) -> Result<u64> {
        let mut total_size = 0u64;

        for path in &input.paths {
            if let Ok(metadata) = std::fs::metadata(path) {
                // Estimate in-memory size as roughly 2x file size
                // (accounting for data structures overhead)
                total_size += metadata.len() * 2;
            }
        }

        // Add overhead for processing structures
        total_size += 100 * 1024 * 1024; // 100MB overhead

        Ok(total_size)
    }

    /// Check if the dataset can fit in available memory
    fn check_memory_constraints(&self, estimated_usage: u64) -> Result<()> {
        if self.config.memory_limit > 0 && estimated_usage > self.config.memory_limit {
            return Err(ProcessingError::SystemError(format!(
                "Insufficient memory: estimated usage {} exceeds limit {}",
                estimated_usage, self.config.memory_limit
            ))
            .into());
        }

        // Check available system memory (simplified)
        let available_memory = self.get_available_memory();
        if estimated_usage > available_memory * 80 / 100 {
            // Use max 80% of available memory
            return Err(ProcessingError::SystemError(format!(
                "Insufficient memory: estimated usage {estimated_usage} exceeds 80% of available memory {available_memory}"
            ))
            .into());
        }

        Ok(())
    }

    /// Get available system memory (simplified implementation)
    fn get_available_memory(&self) -> u64 {
        // This is a simplified implementation
        // In practice, you'd use a system info crate
        8 * 1024 * 1024 * 1024 // Assume 8GB available
    }
}

#[async_trait]
impl DataProcessor for InMemoryProcessor {
    fn config(&self) -> &ProcessingConfig {
        &self.config
    }

    fn stats(&self) -> &ProcessingStats {
        &self.stats
    }

    fn reset_stats(&mut self) {
        self.stats = ProcessingStats::default();
    }

    fn can_process(&self, input: &ProcessingInput) -> Result<bool> {
        // Check if we can estimate memory usage
        let estimated_usage = self.estimate_memory_usage_internal(input)?;

        // Check memory constraints
        match self.check_memory_constraints(estimated_usage) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false), // Can't process due to memory constraints
        }
    }

    async fn process(
        &mut self,
        input: ProcessingInput,
        output: ProcessingOutput,
    ) -> Result<ProcessingContext> {
        let start_time = Instant::now();
        self.reset_stats();

        // Create processing context
        let mut context = ProcessingContext::new(self.config.clone());
        context.input_paths = input.paths.clone();
        context.output_paths = output.paths.clone();

        // Check memory constraints
        let estimated_usage = self.estimate_memory_usage_internal(&input)?;
        self.check_memory_constraints(estimated_usage)?;

        // Load all data into memory
        let mut dataset = self.load_all_data(&input).await?;

        // Process data in parallel
        self.process_data_parallel(&mut dataset)?;

        // Write processed data
        self.write_processed_data(&dataset, &output).await?;

        // Update context with final stats
        context.stats = self.stats.clone();
        context.stats.processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(context)
    }

    fn supported_strategies(&self) -> Vec<ProcessingStrategy> {
        vec![ProcessingStrategy::InMemory]
    }

    fn estimate_memory_usage(&self, input: &ProcessingInput) -> Result<u64> {
        self.estimate_memory_usage_internal(input)
    }

    fn validate_config(&self, config: &ProcessingConfig) -> Result<()> {
        if config.batch_size == 0 {
            return Err(ProcessingError::InvalidConfiguration(
                "Batch size must be greater than 0".to_string(),
            )
            .into());
        }

        if config.max_threads == 0 {
            return Err(ProcessingError::InvalidConfiguration(
                "Max threads must be greater than 0".to_string(),
            )
            .into());
        }

        Ok(())
    }
}

/// In-memory dataset container
#[derive(Debug, Default)]
struct InMemoryDataSet {
    series: Vec<Series>,
    observations: Vec<Observation>,
    lookups: Vec<Lookup>,
    surveys: Vec<Survey>,
}

impl InMemoryDataSet {
    fn new() -> Self {
        Self::default()
    }

    fn total_records(&self) -> usize {
        self.series.len() + self.observations.len() + self.lookups.len() + self.surveys.len()
    }

    fn is_empty(&self) -> bool {
        self.total_records() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_processor_creation() {
        let config = ProcessingConfig::default();
        let processor = InMemoryProcessor::new(config);
        assert_eq!(
            processor.supported_strategies(),
            vec![ProcessingStrategy::InMemory]
        );
    }

    #[test]
    fn test_config_validation() {
        let processor = InMemoryProcessor::new(ProcessingConfig::default());

        let valid_config = ProcessingConfig::default();
        assert!(processor.validate_config(&valid_config).is_ok());

        let mut invalid_config = ProcessingConfig::default();
        invalid_config.batch_size = 0;
        assert!(processor.validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_data_type_determination() {
        let processor = InMemoryProcessor::new(ProcessingConfig::default());

        assert_eq!(
            processor.determine_data_type("test.series", &None).unwrap(),
            "series"
        );
        assert_eq!(
            processor.determine_data_type("test.data.0", &None).unwrap(),
            "observations"
        );
        assert_eq!(
            processor.determine_data_type("test.area", &None).unwrap(),
            "lookups"
        );

        let hint = Some("custom".to_string());
        assert_eq!(
            processor.determine_data_type("test.txt", &hint).unwrap(),
            "custom"
        );
    }

    #[test]
    fn test_memory_estimation() {
        let processor = InMemoryProcessor::new(ProcessingConfig::default());
        let input = ProcessingInput::new(vec!["nonexistent.csv".to_string()]);

        // Should handle non-existent files gracefully
        let result = processor.estimate_memory_usage(&input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_in_memory_dataset() {
        let mut dataset = InMemoryDataSet::new();
        assert!(dataset.is_empty());
        assert_eq!(dataset.total_records(), 0);

        // Add some test data
        dataset.series.push(Series::new(
            "TEST001",
            "Test",
        ));
        assert!(!dataset.is_empty());
        assert_eq!(dataset.total_records(), 1);
    }
}
