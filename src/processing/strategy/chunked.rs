//! Chunked processing strategy for BLS data processing
//!
//! This module provides a chunked processing strategy that processes data in
//! configurable chunks to balance memory usage and performance. It's optimized
//! for medium-sized datasets that are too large for in-memory processing but
//! don't require memory mapping.

use std::time::Instant;
use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use rayon::prelude::*;
use tokio::task;

use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::data::reader::{create_optimized_reader, DataReader, SeriesReader, ObservationReader, LookupReader, SurveyReader};
use crate::data::writer::{create_optimized_writer, DataWriter, SeriesWriter, ObservationWriter, LookupWriter, SurveyWriter};
use crate::processing::traits::{
    DataProcessor, ProcessingConfig, ProcessingStats, ProcessingStrategy,
    ProcessingInput, ProcessingOutput, ProcessingContext,
};
use crate::error::types::{ProcessingError, Result};
use crate::utils::validation::BLSValidationRules;

/// Chunked processing strategy implementation
pub struct ChunkedProcessor {
    config: ProcessingConfig,
    stats: ProcessingStats,
    validation_rules: BLSValidationRules,
    chunk_size: usize,
}

impl ChunkedProcessor {
    /// Create a new chunked processor with the given configuration
    pub fn new(config: ProcessingConfig) -> Self {
        let chunk_size = config.batch_size.max(1000); // Ensure minimum chunk size
        Self {
            config,
            stats: ProcessingStats::default(),
            validation_rules: BLSValidationRules::default(),
            chunk_size,
        }
    }

    /// Create a chunked processor with custom chunk size
    pub fn with_chunk_size(mut config: ProcessingConfig, chunk_size: usize) -> Self {
        config.batch_size = chunk_size;
        Self {
            config,
            stats: ProcessingStats::default(),
            validation_rules: BLSValidationRules::default(),
            chunk_size,
        }
    }

    /// Process data in chunks from input to output
    async fn process_chunked(&mut self, input: &ProcessingInput, output: &ProcessingOutput) -> Result<()> {
        let start_time = Instant::now();

        // Process each input file
        for (input_idx, input_path) in input.paths.iter().enumerate() {
            let output_path = output.paths.get(input_idx)
                .or_else(|| output.paths.first())
                .ok_or_else(|| ProcessingError::invalid_configuration(
                    "No output path available".to_string()
                ))?;

            self.process_file_chunked(input_path, output_path, input, output).await?;
        }

        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;
        Ok(())
    }

    /// Process a single file in chunks
    async fn process_file_chunked(
        &mut self,
        input_path: &str,
        output_path: &str,
        input: &ProcessingInput,
        output: &ProcessingOutput,
    ) -> Result<()> {
        let input_path_obj = Path::new(input_path);
        let output_path_obj = Path::new(output_path);

        // Open input reader
        let mut reader = create_optimized_reader(input_path_obj)?;
        reader.open(input_path_obj).await?;

        // Open output writer
        let mut writer = create_optimized_writer(output_path_obj)?;
        writer.open(output_path_obj).await?;

        // Determine data type
        let data_type = self.determine_data_type(input_path, &input.format_hint)?;

        // Process data based on type
        match data_type.as_str() {
            "series" => {
                self.process_series_chunked(&mut reader, &mut writer).await?;
            }
            "observations" => {
                self.process_observations_chunked(&mut reader, &mut writer).await?;
            }
            "lookups" => {
                self.process_lookups_chunked(&mut reader, &mut writer).await?;
            }
            "survey" => {
                self.process_survey_chunked(&mut reader, &mut writer).await?;
            }
            _ => {
                return Err(ProcessingError::invalid_configuration((
                    format!("Unknown data type: {}", data_type)
                ).into()));
            }
        }

        // Update statistics
        self.stats.bytes_processed += reader.stats().bytes_processed;
        self.stats.bytes_processed += writer.stats().bytes_written;

        // Close files
        reader.close().await?;
        writer.close().await?;

        Ok(())
    }

    /// Process series data in chunks
    async fn process_series_chunked(
        &mut self,
        reader: &mut Box<dyn DataReader>,
        writer: &mut Box<dyn DataWriter>,
    ) -> Result<()> {
        // This is a simplified implementation
        // In practice, we'd need proper trait object handling for SeriesReader/SeriesWriter
        
        let mut chunk_count = 0;
        let mut total_processed = 0;

        // Simulate chunked processing
        loop {
            // Read a chunk of data (placeholder)
            let chunk = self.read_series_chunk(reader).await?;
            if chunk.is_empty() {
                break;
            }

            // Process the chunk in parallel
            let processed_chunk = self.process_series_chunk(chunk)?;

            // Write the processed chunk
            self.write_series_chunk(writer, processed_chunk).await?;

            chunk_count += 1;
            total_processed += self.chunk_size;

            // Update progress
            if chunk_count % 10 == 0 {
                println!("Processed {} chunks ({} records)", chunk_count, total_processed);
            }
        }

        self.stats.records_processed += total_processed as u64;
        Ok(())
    }

    /// Process observations data in chunks
    async fn process_observations_chunked(
        &mut self,
        reader: &mut Box<dyn DataReader>,
        writer: &mut Box<dyn DataWriter>,
    ) -> Result<()> {
        // Similar to process_series_chunked but for observations
        let mut chunk_count = 0;
        let mut total_processed = 0;

        loop {
            let chunk = self.read_observations_chunk(reader).await?;
            if chunk.is_empty() {
                break;
            }

            let processed_chunk = self.process_observations_chunk(chunk)?;
            self.write_observations_chunk(writer, processed_chunk).await?;

            chunk_count += 1;
            total_processed += self.chunk_size;

            if chunk_count % 10 == 0 {
                println!("Processed {} chunks ({} records)", chunk_count, total_processed);
            }
        }

        self.stats.records_processed += total_processed as u64;
        Ok(())
    }

    /// Process lookups data in chunks
    async fn process_lookups_chunked(
        &mut self,
        reader: &mut Box<dyn DataReader>,
        writer: &mut Box<dyn DataWriter>,
    ) -> Result<()> {
        // Similar to process_series_chunked but for lookups
        let mut chunk_count = 0;
        let mut total_processed = 0;

        loop {
            let chunk = self.read_lookups_chunk(reader).await?;
            if chunk.is_empty() {
                break;
            }

            let processed_chunk = self.process_lookups_chunk(chunk)?;
            self.write_lookups_chunk(writer, processed_chunk).await?;

            chunk_count += 1;
            total_processed += self.chunk_size;

            if chunk_count % 10 == 0 {
                println!("Processed {} chunks ({} records)", chunk_count, total_processed);
            }
        }

        self.stats.records_processed += total_processed as u64;
        Ok(())
    }

    /// Process survey data (typically small, no chunking needed)
    async fn process_survey_chunked(
        &mut self,
        reader: &mut Box<dyn DataReader>,
        writer: &mut Box<dyn DataWriter>,
    ) -> Result<()> {
        // Surveys are typically small, so we process them as a single unit
        let survey = self.read_survey(reader).await?;
        let processed_survey = self.process_survey(survey)?;
        self.write_survey(writer, processed_survey).await?;

        self.stats.records_processed += 1;
        Ok(())
    }

    /// Read a chunk of series data (placeholder implementation)
    async fn read_series_chunk(&self, reader: &mut Box<dyn DataReader>) -> Result<Vec<Series>> {
        // This would be implemented with proper async trait object handling
        // For now, return empty vector to indicate end of data
        Ok(Vec::new())
    }

    /// Read a chunk of observations data (placeholder implementation)
    async fn read_observations_chunk(&self, reader: &mut Box<dyn DataReader>) -> Result<Vec<Observation>> {
        // This would be implemented with proper async trait object handling
        Ok(Vec::new())
    }

    /// Read a chunk of lookups data (placeholder implementation)
    async fn read_lookups_chunk(&self, reader: &mut Box<dyn DataReader>) -> Result<Vec<Lookup>> {
        // This would be implemented with proper async trait object handling
        Ok(Vec::new())
    }

    /// Read survey data (placeholder implementation)
    async fn read_survey(&self, reader: &mut Box<dyn DataReader>) -> Result<Survey> {
        // This would be implemented with proper async trait object handling
        Err(ProcessingError::system_error(
            "Survey reading not yet implemented".to_string()
        ).into())
    }

    /// Process a chunk of series data in parallel
    fn process_series_chunk(&self, mut chunk: Vec<Series>) -> Result<Vec<Series>> {
        let start_time = Instant::now();

        // Process chunk in parallel using Rayon
        chunk.par_iter_mut().for_each(|series| {
            self.process_single_series(series);
        });

        // Filter out invalid records if configured
        if self.config.validate_data {
            chunk.retain(|series| self.is_valid_series(series));
        }

        Ok(chunk)
    }

    /// Process a chunk of observations data in parallel
    fn process_observations_chunk(&self, mut chunk: Vec<Observation>) -> Result<Vec<Observation>> {
        let start_time = Instant::now();

        chunk.par_iter_mut().for_each(|observation| {
            self.process_single_observation(observation);
        });

        if self.config.validate_data {
            chunk.retain(|observation| self.is_valid_observation(observation));
        }

        Ok(chunk)
    }

    /// Process a chunk of lookups data in parallel
    fn process_lookups_chunk(&self, mut chunk: Vec<Lookup>) -> Result<Vec<Lookup>> {
        let start_time = Instant::now();

        chunk.par_iter_mut().for_each(|lookup| {
            self.process_single_lookup(lookup);
        });

        if self.config.validate_data {
            chunk.retain(|lookup| self.is_valid_lookup(lookup));
        }

        Ok(chunk)
    }

    /// Process a single survey
    fn process_survey(&self, mut survey: Survey) -> Result<Survey> {
        // Apply transformations and validations
        Ok(survey)
    }

    /// Process a single series record
    fn process_single_series(&self, series: &mut Series) {
        // Apply transformations, validations, etc.
        // This is where business logic would be applied
    }

    /// Process a single observation record
    fn process_single_observation(&self, observation: &mut Observation) {
        // Apply transformations, validations, etc.
    }

    /// Process a single lookup record
    fn process_single_lookup(&self, lookup: &mut Lookup) {
        // Apply transformations, validations, etc.
    }

    /// Validate a series record
    fn is_valid_series(&self, series: &Series) -> bool {
        if let Ok(_) = self.validation_rules.validate_series_record(&[
            series.series_id.to_string(),
            series.title.to_string(),
        ]) {
            true
        } else {
            false
        }
    }

    /// Validate an observation record
    fn is_valid_observation(&self, observation: &Observation) -> bool {
        if let Ok(_) = self.validation_rules.validate_observation_record(&[
            observation.series_id().to_string(),
            observation.year().to_string(),
            observation.period().to_string(),
        ]) {
            true
        } else {
            false
        }
    }

    /// Validate a lookup record
    fn is_valid_lookup(&self, lookup: &Lookup) -> bool {
        if let Ok(_) = self.validation_rules.validate_lookup_record(&[
            lookup.code().to_string(),
            lookup.name().to_string(),
        ]) {
            true
        } else {
            false
        }
    }

    /// Write a chunk of series data (placeholder implementation)
    async fn write_series_chunk(&self, writer: &mut Box<dyn DataWriter>, chunk: Vec<Series>) -> Result<()> {
        // This would be implemented with proper async trait object handling
        Ok(())
    }

    /// Write a chunk of observations data (placeholder implementation)
    async fn write_observations_chunk(&self, writer: &mut Box<dyn DataWriter>, chunk: Vec<Observation>) -> Result<()> {
        // This would be implemented with proper async trait object handling
        Ok(())
    }

    /// Write a chunk of lookups data (placeholder implementation)
    async fn write_lookups_chunk(&self, writer: &mut Box<dyn DataWriter>, chunk: Vec<Lookup>) -> Result<()> {
        // This would be implemented with proper async trait object handling
        Ok(())
    }

    /// Write survey data (placeholder implementation)
    async fn write_survey(&self, writer: &mut Box<dyn DataWriter>, survey: Survey) -> Result<()> {
        // This would be implemented with proper async trait object handling
        Ok(())
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

    /// Estimate memory usage for chunked processing
    fn estimate_memory_usage_internal(&self, input: &ProcessingInput) -> Result<u64> {
        // For chunked processing, memory usage is primarily determined by chunk size
        let chunk_memory = self.chunk_size as u64 * 1024; // Estimate 1KB per record
        let overhead = 50 * 1024 * 1024; // 50MB overhead
        
        Ok(chunk_memory + overhead)
    }

    /// Calculate optimal chunk size based on available memory
    pub fn calculate_optimal_chunk_size(&self, available_memory: u64, record_size_estimate: u64) -> usize {
        let target_memory_usage = available_memory / 4; // Use 25% of available memory
        let optimal_chunk_size = target_memory_usage / record_size_estimate.max(1024);
        
        // Ensure chunk size is within reasonable bounds
        optimal_chunk_size.max(100).min(100_000) as usize
    }
}

#[async_trait]
impl DataProcessor for ChunkedProcessor {
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
        // Chunked processing can handle most datasets
        // Check if files exist and are readable
        for path in &input.paths {
            if !Path::new(path).exists() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn process(&mut self, input: ProcessingInput, output: ProcessingOutput) -> Result<ProcessingContext> {
        let start_time = Instant::now();
        self.reset_stats();

        // Create processing context
        let mut context = ProcessingContext::new(self.config.clone());
        context.input_paths = input.paths.clone();
        context.output_paths = output.paths.clone();

        // Process data in chunks
        self.process_chunked(&input, &output).await?;

        // Update context with final stats
        context.stats = self.stats.clone();
        context.stats.processing_time_ms = start_time.elapsed().as_millis() as u64;
        context.stats.threads_used = rayon::current_num_threads();

        Ok(context)
    }

    fn supported_strategies(&self) -> Vec<ProcessingStrategy> {
        vec![ProcessingStrategy::Chunked]
    }

    fn estimate_memory_usage(&self, input: &ProcessingInput) -> Result<u64> {
        self.estimate_memory_usage_internal(input)
    }

    fn validate_config(&self, config: &ProcessingConfig) -> Result<()> {
        if config.batch_size == 0 {
            return Err(ProcessingError::system_error(
                "Batch size must be greater than 0".to_string()
            ).into());
        }

        if config.max_threads == 0 {
            return Err(ProcessingError::system_error(
                "Max threads must be greater than 0".to_string()
            ).into());
        }

        if config.buffer_size == 0 {
            return Err(ProcessingError::system_error((
                "Buffer size must be greater than 0".to_string()
            ).into()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunked_processor_creation() {
        let config = ProcessingConfig::default();
        let processor = ChunkedProcessor::new(config);
        assert_eq!(processor.supported_strategies(), vec![ProcessingStrategy::Chunked]);
        assert!(processor.chunk_size >= 1000);
    }

    #[test]
    fn test_chunked_processor_with_custom_chunk_size() {
        let config = ProcessingConfig::default();
        let processor = ChunkedProcessor::with_chunk_size(config, 5000);
        assert_eq!(processor.chunk_size, 5000);
    }

    #[test]
    fn test_config_validation() {
        let processor = ChunkedProcessor::new(ProcessingConfig::default());
        
        let valid_config = ProcessingConfig::default();
        assert!(processor.validate_config(&valid_config).is_ok());
        
        let mut invalid_config = ProcessingConfig::default();
        invalid_config.batch_size = 0;
        assert!(processor.validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_data_type_determination() {
        let processor = ChunkedProcessor::new(ProcessingConfig::default());
        
        assert_eq!(processor.determine_data_type("test.series", &None).unwrap(), "series");
        assert_eq!(processor.determine_data_type("test.data.0", &None).unwrap(), "observations");
        assert_eq!(processor.determine_data_type("test.area", &None).unwrap(), "lookups");
        
        let hint = Some("custom".to_string());
        assert_eq!(processor.determine_data_type("test.txt", &hint).unwrap(), "custom");
    }

    #[test]
    fn test_memory_estimation() {
        let processor = ChunkedProcessor::new(ProcessingConfig::default());
        let input = ProcessingInput::new(vec!["test.csv".to_string()]);
        
        let result = processor.estimate_memory_usage(&input);
        assert!(result.is_ok());
        
        let memory_usage = result.unwrap();
        assert!(memory_usage > 0);
    }

    #[test]
    fn test_optimal_chunk_size_calculation() {
        let processor = ChunkedProcessor::new(ProcessingConfig::default());
        
        let available_memory = 1024 * 1024 * 1024; // 1GB
        let record_size = 1024; // 1KB per record
        
        let optimal_size = processor.calculate_optimal_chunk_size(available_memory, record_size);
        assert!(optimal_size >= 100);
        assert!(optimal_size <= 100_000);
    }

    #[test]
    fn test_can_process() {
        let processor = ChunkedProcessor::new(ProcessingConfig::default());
        
        // Test with non-existent file
        let input = ProcessingInput::new(vec!["nonexistent.csv".to_string()]);
        assert!(!processor.can_process(&input).unwrap());
        
        // Test with empty input
        let empty_input = ProcessingInput::new(vec![]);
        assert!(processor.can_process(&empty_input).unwrap());
    }

    #[test]
    fn test_validation_methods() {
        let processor = ChunkedProcessor::new(ProcessingConfig::default());
        
        let series = Series::new(&*"TEST001".to_string(), &*"Test".to_string(), "AREA001".to_string());
        // This would test validation if validation rules were properly implemented
        // For now, just ensure the method doesn't panic
        let _is_valid = processor.is_valid_series(&series);
    }
}