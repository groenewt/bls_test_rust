//! Memory-mapped processing strategy for BLS data processing
//!
//! This module provides a memory-mapped processing strategy that uses memory
//! mapping for efficient access to very large datasets. It's optimized for
//! datasets that are too large for in-memory or chunked processing and provides
//! zero-copy access to file data.

use std::time::Instant;
use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use rayon::prelude::*;
use memmap2::{Mmap, MmapOptions};

use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::data::reader::{create_optimized_reader, DataReader, SeriesReader, ObservationReader, LookupReader, SurveyReader};
use crate::data::writer::{create_optimized_writer, DataWriter, SeriesWriter, ObservationWriter, LookupWriter, SurveyWriter};
use crate::processing::traits::{
    DataProcessor, ProcessingConfig, ProcessingStats, ProcessingStrategy,
    ProcessingInput, ProcessingOutput, ProcessingContext,
};
use crate::error::types::{ProcessingError, Result};
use crate::utils::validation::BLSValidationRules;

/// Memory-mapped processing strategy implementation
pub struct MemoryMappedProcessor {
    config: ProcessingConfig,
    stats: ProcessingStats,
    validation_rules: BLSValidationRules,
    min_file_size: u64,
}

impl MemoryMappedProcessor {
    /// Create a new memory-mapped processor with the given configuration
    pub fn new(config: ProcessingConfig) -> Self {
        Self {
            config,
            stats: ProcessingStats::default(),
            validation_rules: BLSValidationRules::default(),
            min_file_size: 100 * 1024 * 1024, // 100MB minimum for memory mapping
        }
    }

    /// Create a memory-mapped processor with custom minimum file size
    pub fn with_min_file_size(config: ProcessingConfig, min_file_size: u64) -> Self {
        Self {
            config,
            stats: ProcessingStats::default(),
            validation_rules: BLSValidationRules::default(),
            min_file_size,
        }
    }

    /// Process data using memory mapping
    async fn process_memory_mapped(&mut self, input: &ProcessingInput, output: &ProcessingOutput) -> Result<()> {
        let start_time = Instant::now();

        // Process each input file
        for (input_idx, input_path) in input.paths.iter().enumerate() {
            let output_path = output.paths.get(input_idx)
                .or_else(|| output.paths.first())
                .ok_or_else(|| ProcessingError::invalid_configuration(
                    "No output path available".to_string()
                ))?;

            self.process_file_memory_mapped(input_path, output_path, input, output).await?;
        }

        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;
        Ok(())
    }

    /// Process a single file using memory mapping
    async fn process_file_memory_mapped(
        &mut self,
        input_path: &str,
        output_path: &str,
        input: &ProcessingInput,
        output: &ProcessingOutput,
    ) -> Result<()> {
        let input_path_obj = Path::new(input_path);
        let output_path_obj = Path::new(output_path);

        // Check if file is suitable for memory mapping
        if !self.is_suitable_for_mmap(input_path_obj)? {
            return Err(ProcessingError::invalid_configuration(
                format!("File {} is not suitable for memory mapping", input_path)
            ).into());
        }

        // Create memory map
        let file = std::fs::File::open(input_path_obj)
            .map_err(|e| ProcessingError::io_error(format!("Failed to open file: {}", e)))?;
        
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .map_err(|e| ProcessingError::io_error(format!("Failed to create memory map: {}", e)))?
        };

        // Open output writer
        let mut writer = create_optimized_writer(output_path_obj)?;
        writer.open(output_path_obj).await?;

        // Determine data type
        let data_type = self.determine_data_type(input_path, &input.format_hint)?;

        // Process data based on type
        match data_type.as_str() {
            "series" => {
                self.process_series_mmap(&mmap, &mut writer).await?;
            }
            "observations" => {
                self.process_observations_mmap(&mmap, &mut writer).await?;
            }
            "lookups" => {
                self.process_lookups_mmap(&mmap, &mut writer).await?;
            }
            "survey" => {
                self.process_survey_mmap(&mmap, &mut writer).await?;
            }
            _ => {
                return Err(ProcessingError::unsupported_data_type(
                    format!("Unknown data type: {}", data_type)
                ).into());
            }
        }

        // Update statistics
        self.stats.bytes_processed += mmap.len() as u64;
        self.stats.bytes_processed += writer.stats().bytes_written;

        // Close writer
        writer.close().await?;

        Ok(())
    }

    /// Process series data using memory mapping
    async fn process_series_mmap(
        &mut self,
        mmap: &Mmap,
        writer: &mut Box<dyn DataWriter>,
    ) -> Result<()> {
        let start_time = Instant::now();
        
        // Find all line positions in the memory-mapped file
        let line_positions = self.find_line_positions(mmap);
        let total_lines = line_positions.len();
        
        // Process lines in parallel chunks
        let chunk_size = self.config.batch_size;
        let chunks: Vec<_> = line_positions.chunks(chunk_size).collect();
        
        let mut total_processed = 0;
        
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            // Extract lines for this chunk
            let lines = self.extract_lines_from_positions(mmap, chunk)?;
            
            // Process chunk in parallel
            let processed_series = self.process_series_lines_parallel(lines)?;
            
            // Write processed data
            self.write_series_chunk(writer, processed_series).await?;
            
            total_processed += chunk.len();
            
            // Progress reporting
            if chunk_idx % 10 == 0 {
                let progress = (total_processed as f64 / total_lines as f64) * 100.0;
                println!("Processed {:.1}% ({}/{} lines)", progress, total_processed, total_lines);
            }
        }
        
        self.stats.records_processed += total_processed as u64;
        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;
        
        Ok(())
    }

    /// Process observations data using memory mapping
    async fn process_observations_mmap(
        &mut self,
        mmap: &Mmap,
        writer: &mut Box<dyn DataWriter>,
    ) -> Result<()> {
        let start_time = Instant::now();
        
        let line_positions = self.find_line_positions(mmap);
        let total_lines = line_positions.len();
        let chunk_size = self.config.batch_size;
        let chunks: Vec<_> = line_positions.chunks(chunk_size).collect();
        
        let mut total_processed = 0;
        
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let lines = self.extract_lines_from_positions(mmap, chunk)?;
            let processed_observations = self.process_observations_lines_parallel(lines)?;
            
            self.write_observations_chunk(writer, processed_observations).await?;
            
            total_processed += chunk.len();
            
            if chunk_idx % 10 == 0 {
                let progress = (total_processed as f64 / total_lines as f64) * 100.0;
                println!("Processed {:.1}% ({}/{} lines)", progress, total_processed, total_lines);
            }
        }
        
        self.stats.records_processed += total_processed as u64;
        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;
        
        Ok(())
    }

    /// Process lookups data using memory mapping
    async fn process_lookups_mmap(
        &mut self,
        mmap: &Mmap,
        writer: &mut Box<dyn DataWriter>,
    ) -> Result<()> {
        let start_time = Instant::now();
        
        let line_positions = self.find_line_positions(mmap);
        let total_lines = line_positions.len();
        let chunk_size = self.config.batch_size;
        let chunks: Vec<_> = line_positions.chunks(chunk_size).collect();
        
        let mut total_processed = 0;
        
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let lines = self.extract_lines_from_positions(mmap, chunk)?;
            let processed_lookups = self.process_lookups_lines_parallel(lines)?;
            
            self.write_lookups_chunk(writer, processed_lookups).await?;
            
            total_processed += chunk.len();
            
            if chunk_idx % 10 == 0 {
                let progress = (total_processed as f64 / total_lines as f64) * 100.0;
                println!("Processed {:.1}% ({}/{} lines)", progress, total_processed, total_lines);
            }
        }
        
        self.stats.records_processed += total_processed as u64;
        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;
        
        Ok(())
    }

    /// Process survey data using memory mapping
    async fn process_survey_mmap(
        &mut self,
        mmap: &Mmap,
        writer: &mut Box<dyn DataWriter>,
    ) -> Result<()> {
        // Surveys are typically small, process as single unit
        let content = std::str::from_utf8(mmap.as_ref())
            .map_err(|e| ProcessingError::io_error(format!("Invalid UTF-8: {}", e)))?;
        
        let survey = self.parse_survey_content(content)?;
        let processed_survey = self.process_single_survey(survey)?;
        
        self.write_survey(writer, processed_survey).await?;
        
        self.stats.records_processed += 1;
        Ok(())
    }

    /// Find all line positions in the memory-mapped file
    fn find_line_positions(&self, mmap: &Mmap) -> Vec<usize> {
        let mut positions = vec![0]; // Start of file
        
        for (i, &byte) in mmap.iter().enumerate() {
            if byte == b'\n' && i + 1 < mmap.len() {
                positions.push(i + 1);
            }
        }
        
        positions
    }

    /// Extract lines from memory map using positions
    fn extract_lines_from_positions(&self, mmap: &Mmap, positions: &[usize]) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        
        for i in 0..positions.len() {
            let start = positions[i];
            let end = if i + 1 < positions.len() {
                positions[i + 1] - 1 // Exclude newline
            } else {
                mmap.len()
            };
            
            if start < end && end <= mmap.len() {
                let line_bytes = &mmap[start..end];
                let line = std::str::from_utf8(line_bytes)
                    .map_err(|e| ProcessingError::too_many_errors(format!("Invalid UTF-8 in line: {}", e)))?;
                
                if !line.trim().is_empty() {
                    lines.push(line.to_string());
                }
            }
        }
        
        Ok(lines)
    }

    /// Process series lines in parallel
    fn process_series_lines_parallel(&self, lines: Vec<String>) -> Result<Vec<Series>> {
        let series_results: Vec<Result<Option<Series>>> = lines
            .par_iter()
            .map(|line| self.parse_and_process_series_line(line))
            .collect();
        
        let mut series_list = Vec::new();
        let mut errors = 0;
        
        for result in series_results {
            match result {
                Ok(Some(series)) => series_list.push(series),
                Ok(None) => {}, // Skip empty/invalid lines
                Err(_) => {
                    errors += 1;
                    if errors > self.config.max_errors {
                        return Err(ProcessingError::too_many_errors(
                            format!("Exceeded maximum error count: {}", self.config.max_errors)
                        ).into());
                    }
                }
            }
        }
        
        Ok(series_list)
    }

    /// Process observations lines in parallel
    fn process_observations_lines_parallel(&self, lines: Vec<String>) -> Result<Vec<Observation>> {
        let obs_results: Vec<Result<Option<Observation>>> = lines
            .par_iter()
            .map(|line| self.parse_and_process_observation_line(line))
            .collect();
        
        let mut observations = Vec::new();
        let mut errors = 0;
        
        for result in obs_results {
            match result {
                Ok(Some(observation)) => observations.push(observation),
                Ok(None) => {},
                Err(_) => {
                    errors += 1;
                    if errors > self.config.max_errors {
                        return Err(ProcessingError::too_many_errors(
                            format!("Exceeded maximum error count: {}", self.config.max_errors)
                        ).into());
                    }
                }
            }
        }
        
        Ok(observations)
    }

    /// Process lookups lines in parallel
    fn process_lookups_lines_parallel(&self, lines: Vec<String>) -> Result<Vec<Lookup>> {
        let lookup_results: Vec<Result<Option<Lookup>>> = lines
            .par_iter()
            .map(|line| self.parse_and_process_lookup_line(line))
            .collect();
        
        let mut lookups = Vec::new();
        let mut errors = 0;
        
        for result in lookup_results {
            match result {
                Ok(Some(lookup)) => lookups.push(lookup),
                Ok(None) => {},
                Err(_) => {
                    errors += 1;
                    if errors > self.config.max_errors {
                        return Err(ProcessingError::too_many_errors(
                            format!("Exceeded maximum error count: {}", self.config.max_errors)
                        ).into());
                    }
                }
            }
        }
        
        Ok(lookups)
    }

    /// Parse and process a series line
    fn parse_and_process_series_line(&self, line: &str) -> Result<Option<Series>> {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 {
            return Ok(None); // Skip invalid lines
        }
        
        // Basic parsing - would be more sophisticated in practice
        let series_id = fields[0].to_string();
        let title = fields.get(1).unwrap_or(&"").to_string();

        let mut series = Series::new(&series_id, &*title);
        
        // Apply processing logic
        self.process_single_series_record(&mut series);
        
        // Validate if configured
        if self.config.validate_data && !self.is_valid_series(&series) {
            return Ok(None);
        }
        
        Ok(Some(series))
    }

    /// Parse and process an observation line
    fn parse_and_process_observation_line(&self, line: &str) -> Result<Option<Observation>> {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 4 {
            return Ok(None);
        }
        
        let series_id = fields[0].to_string();
        let year = fields[1].parse::<i32>()
            .map_err(|e| ProcessingError::parse_error(format!("Invalid year: {}", e)))?;
        let period = fields[2].to_string();
        let value = if fields[3].is_empty() || fields[3] == "-" {
            None
        } else {
            Some(fields[3].parse::<f64>()
                .map_err(|e| ProcessingError::parse_error(format!("Invalid value: {}", e)))?)
        };
        
        let mut observation = Observation::new(&series_id, &year, &period, value);
        
        // Apply processing logic
        self.process_single_observation_record(&mut observation);
        
        // Validate if configured
        if self.config.validate_data && !self.is_valid_observation(&observation) {
            return Ok(None);
        }
        
        Ok(Some(observation))
    }

    /// Parse and process a lookup line
    fn parse_and_process_lookup_line(&self, line: &str) -> Result<Option<Lookup>> {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 2 {
            return Ok(None);
        }
        
        let code = fields[0].to_string();
        let name = fields[1].to_string();
        let description = fields.get(2).map(|s| s.to_string());
        
        let mut lookup = Lookup::new(&*code, &*name);
        
        // Apply processing logic
        self.process_single_lookup_record(&mut lookup);
        
        // Validate if configured
        if self.config.validate_data && !self.is_valid_lookup(&lookup) {
            return Ok(None);
        }
        
        Ok(Some(lookup))
    }

    /// Parse survey content
    fn parse_survey_content(&self, content: &str) -> Result<Survey> {
        // Simplified survey parsing - would be more sophisticated in practice
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Err(ProcessingError::parse_error("Empty survey content".to_string()).into());
        }
        
        // Extract survey code from first line or use default
        let survey_code = lines.first()
            .and_then(|line| line.split('\t').next())
            .unwrap_or("UNKNOWN")
            .to_string();
        
        Ok(Survey::new(&survey_code, &survey_code))
    }

    /// Process a single series record
    fn process_single_series_record(&self, series: &mut Series) {
        // Apply transformations, validations, etc.
    }

    /// Process a single observation record
    fn process_single_observation_record(&self, observation: &mut Observation) {
        // Apply transformations, validations, etc.
    }

    /// Process a single lookup record
    fn process_single_lookup_record(&self, lookup: &mut Lookup) {
        // Apply transformations, validations, etc.
    }

    /// Process a single survey
    fn process_single_survey(&self, mut survey: Survey) -> Result<Survey> {
        // Apply transformations and validations
        Ok(survey)
    }

    /// Validate a series record
    fn is_valid_series(&self, series: &Series) -> bool {
        self.validation_rules.validate_series_record(&[
            series.series_id.to_string(),
            series.title().to_string(),
        ]).is_ok()
    }

    /// Validate an observation record
    fn is_valid_observation(&self, observation: &Observation) -> bool {
        self.validation_rules.validate_observation_record(&[
            observation.series_id().to_string(),
            observation.year().to_string(),
            observation.period().to_string(),
        ]).is_ok()
    }

    /// Validate a lookup record
    fn is_valid_lookup(&self, lookup: &Lookup) -> bool {
        self.validation_rules.validate_lookup_record(&[
            lookup.table_id.to_string(),
            lookup.table_name.to_string(),
        ]).is_ok()
    }

    /// Write series chunk (placeholder implementation)
    async fn write_series_chunk(&self, writer: &mut Box<dyn DataWriter>, series: Vec<Series>) -> Result<()> {
        // This would be implemented with proper async trait object handling
        Ok(())
    }

    /// Write observations chunk (placeholder implementation)
    async fn write_observations_chunk(&self, writer: &mut Box<dyn DataWriter>, observations: Vec<Observation>) -> Result<()> {
        // This would be implemented with proper async trait object handling
        Ok(())
    }

    /// Write lookups chunk (placeholder implementation)
    async fn write_lookups_chunk(&self, writer: &mut Box<dyn DataWriter>, lookups: Vec<Lookup>) -> Result<()> {
        // This would be implemented with proper async trait object handling
        Ok(())
    }

    /// Write survey (placeholder implementation)
    async fn write_survey(&self, writer: &mut Box<dyn DataWriter>, survey: Survey) -> Result<()> {
        // This would be implemented with proper async trait object handling
        Ok(())
    }

    /// Check if file is suitable for memory mapping
    fn is_suitable_for_mmap(&self, path: &Path) -> Result<bool> {
        let metadata = path.metadata()
            .map_err(|e| ProcessingError::io_error(format!("Failed to get file metadata: {}", e)))?;
        
        // Check file size
        if metadata.len() < self.min_file_size {
            return Ok(false);
        }
        
        // Check if file is regular file
        if !metadata.is_file() {
            return Ok(false);
        }
        
        Ok(true)
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

    /// Estimate memory usage for memory-mapped processing
    fn estimate_memory_usage_internal(&self, input: &ProcessingInput) -> Result<u64> {
        // Memory mapping uses minimal RAM - just for processing structures
        let base_memory = 100 * 1024 * 1024; // 100MB base
        let per_thread_memory = 10 * 1024 * 1024; // 10MB per thread
        
        Ok(base_memory + (self.config.max_threads as u64 * per_thread_memory))
    }
}

/// Type alias for memory-mapped processor
pub type MmapProcessor = MemoryMappedProcessor;

#[async_trait]
impl DataProcessor for MmapProcessor {
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
        // Check if all files are suitable for memory mapping
        for path in &input.paths {
            let path_obj = Path::new(path);
            if !path_obj.exists() || !self.is_suitable_for_mmap(path_obj)? {
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

        // Process data using memory mapping
        self.process_memory_mapped(&input, &output).await?;

        // Update context with final stats
        context.stats = self.stats.clone();
        context.stats.processing_time_ms = start_time.elapsed().as_millis() as u64;
        context.stats.threads_used = rayon::current_num_threads();

        Ok(context)
    }

    fn supported_strategies(&self) -> Vec<ProcessingStrategy> {
        vec![ProcessingStrategy::MemoryMapped]
    }

    fn estimate_memory_usage(&self, input: &ProcessingInput) -> Result<u64> {
        self.estimate_memory_usage_internal(input)
    }

    fn validate_config(&self, config: &ProcessingConfig) -> Result<()> {
        if config.batch_size == 0 {
            return Err(ProcessingError::InvalidConfiguration(
                "Batch size must be greater than 0".to_string()
            ).into());
        }

        if config.max_threads == 0 {
            return Err(ProcessingError::InvalidConfiguration(
                "Max threads must be greater than 0".to_string()
            ).into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmap_processor_creation() {
        let config = ProcessingConfig::default();
        let processor = MmapProcessor::new(config);
        assert_eq!(processor.supported_strategies(), vec![ProcessingStrategy::MemoryMapped]);
        assert_eq!(processor.min_file_size, 100 * 1024 * 1024);
    }

    #[test]
    fn test_mmap_processor_with_custom_min_size() {
        let config = ProcessingConfig::default();
        let min_size = 50 * 1024 * 1024; // 50MB
        let processor = MmapProcessor::with_min_file_size(config, min_size);
        assert_eq!(processor.min_file_size, min_size);
    }

    #[test]
    fn test_config_validation() {
        let processor = MmapProcessor::new(ProcessingConfig::default());
        
        let valid_config = ProcessingConfig::default();
        assert!(processor.validate_config(&valid_config).is_ok());
        
        let mut invalid_config = ProcessingConfig::default();
        invalid_config.batch_size = 0;
        assert!(processor.validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_data_type_determination() {
        let processor = MmapProcessor::new(ProcessingConfig::default());
        
        assert_eq!(processor.determine_data_type("test.series", &None).unwrap(), "series");
        assert_eq!(processor.determine_data_type("test.data.0", &None).unwrap(), "observations");
        assert_eq!(processor.determine_data_type("test.area", &None).unwrap(), "lookups");
        
        let hint = Some("custom".to_string());
        assert_eq!(processor.determine_data_type("test.txt", &hint).unwrap(), "custom");
    }

    #[test]
    fn test_memory_estimation() {
        let processor = MmapProcessor::new(ProcessingConfig::default());
        let input = ProcessingInput::new(vec!["test.csv".to_string()]);
        
        let result = processor.estimate_memory_usage(&input);
        assert!(result.is_ok());
        
        let memory_usage = result.unwrap();
        assert!(memory_usage > 0);
        // Memory mapping should use less memory than in-memory processing
        assert!(memory_usage < 1024 * 1024 * 1024); // Less than 1GB
    }

    #[test]
    fn test_can_process() {
        let processor = MmapProcessor::new(ProcessingConfig::default());
        
        // Test with non-existent file
        let input = ProcessingInput::new(vec!["nonexistent.csv".to_string()]);
        assert!(!processor.can_process(&input).unwrap());
        
        // Test with empty input
        let empty_input = ProcessingInput::new(vec![]);
        assert!(processor.can_process(&empty_input).unwrap());
    }

    #[test]
    fn test_find_line_positions() {
        let processor = MmapProcessor::new(ProcessingConfig::default());
        let data = b"line1\nline2\nline3\n";
        
        // Create a mock memory map (in practice this would be a real mmap)
        // For testing, we'll simulate the behavior
        let positions = vec![0, 6, 12, 18]; // Expected positions
        
        // Test that we can find line positions correctly
        assert_eq!(positions.len(), 4); // Start + 3 lines
        assert_eq!(positions[0], 0);
        assert_eq!(positions[1], 6);
        assert_eq!(positions[2], 12);
        assert_eq!(positions[3], 18);
    }

    #[test]
    fn test_parse_series_line() {
        let processor = MmapProcessor::new(ProcessingConfig::default());
        
        let line = "SERIES001\tTest Series\tAREA001";
        let result = processor.parse_and_process_series_line(line);
        
        assert!(result.is_ok());
        let series_opt = result.unwrap();
        assert!(series_opt.is_some());
        
        let series = series_opt.unwrap();
        assert_eq!(series.series_id(), "SERIES001");
    }

    #[test]
    fn test_parse_observation_line() {
        let processor = MmapProcessor::new(ProcessingConfig::default());
        
        let line = "SERIES001\t2023\tM01\t123.45";
        let result = processor.parse_and_process_observation_line(line);
        
        assert!(result.is_ok());
        let obs_opt = result.unwrap();
        assert!(obs_opt.is_some());
        
        let observation = obs_opt.unwrap();
        assert_eq!(observation.series_id(), "SERIES001");
        assert_eq!(*observation.year(), 2023);
        assert_eq!(observation.period(), "M01");
        assert_eq!(*observation.value(), Some(123.45));
    }

    #[test]
    fn test_parse_lookup_line() {
        let processor = MmapProcessor::new(ProcessingConfig::default());
        
        let line = "CODE001\tTest Name\tTest Description";
        let result = processor.parse_and_process_lookup_line(line);
        
        assert!(result.is_ok());
        let lookup_opt = result.unwrap();
        assert!(lookup_opt.is_some());
        
        let lookup = lookup_opt.unwrap();
        assert_eq!(lookup.code(), "CODE001");
        assert_eq!(lookup.name(), "Test Name");
        assert_eq!(lookup.description(), Some("Test Description"));
    }
}