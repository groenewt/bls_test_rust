//! # CSV Output Format Implementation
//!
//! This module provides CSV output format support for BLS data.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use async_trait::async_trait;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::output::traits::{FormatWriter, OutputConfig, OutputResult, OutputGenerator, OutputStats};
use crate::processing::traits::ProcessedData;
use crate::error::types::{ProcessingError, Result};

/// CSV format writer implementation
pub struct CsvWriter {
    /// Writer statistics
    stats: OutputStats,
}

impl CsvWriter {
    /// Create a new CSV writer
    pub fn new() -> Self {
        Self {
            stats: OutputStats::default(),
        }
    }

    /// Write CSV header for series data
    fn series_header() -> &'static str {
        "series_id,title,area_code,item_code,seasonal,periodicity_code,base_code,base_period\n"
    }

    /// Write CSV header for observation data
    fn observation_header() -> &'static str {
        "series_id,year,period,value,footnote_codes\n"
    }

    /// Write CSV header for lookup data
    fn lookup_header() -> &'static str {
        "code,text\n"
    }

    /// Write CSV header for survey data
    fn survey_header() -> &'static str {
        "survey_abbreviation,survey_name,begin_year,begin_period,end_year,end_period\n"
    }

    /// Format series data as CSV row
    fn format_series_row(series: &Series) -> String {
        format!(
            "{},{},{},{},{},{},{},{}\n",
            series.series_id,
            series.title,
            series.area_code,
            series.item_code,
            series.seasonal,
            series.periodicity_code,
            series.base_code,
            series.base_period
        )
    }

    /// Format observation data as CSV row
    fn format_observation_row(observation: &Observation) -> String {
        format!(
            "{},{},{},{},{}\n",
            observation.series_id,
            observation.year,
            observation.period,
            observation.value.format_value(None),
            observation.value.footnotes.join(";")
        )
    }

    /// Format lookup data as CSV row
    fn format_lookup_row(lookup: &Lookup) -> String {
        format!("{},{}\n", lookup.table_id, lookup.table_name)
    }

    /// Format survey data as CSV row
    fn format_survey_row(survey: &Survey) -> String {
        format!(
            "{},{},{},{},{},{}\n",
            survey.survey_code,
            survey.name,
            survey.metadata.start_date.map(|d| d.format("%Y").to_string()).unwrap_or("".to_string()),
            "",
            survey.metadata.end_date.map(|d| d.format("%Y").to_string()).unwrap_or("".to_string()),
            ""
        )
    }
}

impl Default for CsvWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FormatWriter for CsvWriter {
    fn format_name(&self) -> &str {
        "csv"
    }

    fn file_extension(&self) -> &str {
        "csv"
    }

    async fn write_series(&mut self, series: &[Series], path: &Path, _config: &OutputConfig) -> Result<OutputResult> {
        let start_time = Instant::now();
        
        let file = File::create(path).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to create file: {}", e)))?;
        let mut writer = BufWriter::new(file);

        // Write header
        writer.write_all(Self::series_header().as_bytes()).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write header: {}", e)))?;

        // Write data rows
        let mut bytes_written = Self::series_header().len() as u64;
        for s in series {
            let row = Self::format_series_row(s);
            writer.write_all(row.as_bytes()).await
                .map_err(|e| ProcessingError::io_error(format!("Failed to write row: {}", e)))?;
            bytes_written += row.len() as u64;
        }

        writer.flush().await
            .map_err(|e| ProcessingError::io_error(format!("Failed to flush writer: {}", e)))?;

        let elapsed = start_time.elapsed();
        let result = OutputResult {
            output_paths: vec![path.to_string_lossy().to_string()],
            records_written: series.len() as u64,
            bytes_written,
            generation_time_ms: elapsed.as_millis() as u64,
            metadata: HashMap::new(),
        };

        self.stats.update(&result, true);
        Ok(result)
    }

    async fn write_observations(&mut self, observations: &[Observation], path: &Path, _config: &OutputConfig) -> Result<OutputResult> {
        let start_time = Instant::now();
        
        let file = File::create(path).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to create file: {}", e)))?;
        let mut writer = BufWriter::new(file);

        // Write header
        writer.write_all(Self::observation_header().as_bytes()).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write header: {}", e)))?;

        // Write data rows
        let mut bytes_written = Self::observation_header().len() as u64;
        for obs in observations {
            let row = Self::format_observation_row(obs);
            writer.write_all(row.as_bytes()).await
                .map_err(|e| ProcessingError::io_error(format!("Failed to write row: {}", e)))?;
            bytes_written += row.len() as u64;
        }

        writer.flush().await
            .map_err(|e| ProcessingError::io_error(format!("Failed to flush writer: {}", e)))?;

        let elapsed = start_time.elapsed();
        let result = OutputResult {
            output_paths: vec![path.to_string_lossy().to_string()],
            records_written: observations.len() as u64,
            bytes_written,
            generation_time_ms: elapsed.as_millis() as u64,
            metadata: HashMap::new(),
        };

        self.stats.update(&result, true);
        Ok(result)
    }

    async fn write_lookups(&mut self, lookups: &[Lookup], path: &Path, _config: &OutputConfig) -> Result<OutputResult> {
        let start_time = Instant::now();
        
        let file = File::create(path).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to create file: {}", e)))?;
        let mut writer = BufWriter::new(file);

        // Write header
        writer.write_all(Self::lookup_header().as_bytes()).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write header: {}", e)))?;

        // Write data rows
        let mut bytes_written = Self::lookup_header().len() as u64;
        for lookup in lookups {
            let row = Self::format_lookup_row(lookup);
            writer.write_all(row.as_bytes()).await
                .map_err(|e| ProcessingError::io_error(format!("Failed to write row: {}", e)))?;
            bytes_written += row.len() as u64;
        }

        writer.flush().await
            .map_err(|e| ProcessingError::io_error(format!("Failed to flush writer: {}", e)))?;

        let elapsed = start_time.elapsed();
        let result = OutputResult {
            output_paths: vec![path.to_string_lossy().to_string()],
            records_written: lookups.len() as u64,
            bytes_written,
            generation_time_ms: elapsed.as_millis() as u64,
            metadata: HashMap::new(),
        };

        self.stats.update(&result, true);
        Ok(result)
    }

    async fn write_survey(&mut self, survey: &Survey, path: &Path, _config: &OutputConfig) -> Result<OutputResult> {
        let start_time = Instant::now();
        
        let file = File::create(path).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to create file: {}", e)))?;
        let mut writer = BufWriter::new(file);

        // Write header
        writer.write_all(Self::survey_header().as_bytes()).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write header: {}", e)))?;

        // Write data row
        let row = Self::format_survey_row(survey);
        writer.write_all(row.as_bytes()).await
            .map_err(|e| ProcessingError::io_error(format!("Failed to write row: {}", e)))?;

        writer.flush().await
            .map_err(|e| ProcessingError::io_error(format!("Failed to flush writer: {}", e)))?;

        let bytes_written = (Self::survey_header().len() + row.len()) as u64;
        let elapsed = start_time.elapsed();
        let result = OutputResult {
            output_paths: vec![path.to_string_lossy().to_string()],
            records_written: 1,
            bytes_written,
            generation_time_ms: elapsed.as_millis() as u64,
            metadata: HashMap::new(),
        };

        self.stats.update(&result, true);
        Ok(result)
    }

    async fn write_mixed(&mut self, data: ProcessedData, path: &Path, config: &OutputConfig) -> Result<OutputResult> {
        match data {
            ProcessedData::Series(series) => self.write_series(&series, path, config).await,
            ProcessedData::Observations(observations) => self.write_observations(&observations, path, config).await,
            ProcessedData::Lookups(lookups) => self.write_lookups(&lookups, path, config).await,
            ProcessedData::Survey(survey) => self.write_survey(&survey, path, config).await,
            ProcessedData::Mixed { series, observations, lookups, surveys } => {
                // For mixed data, we'll write separate files
                let mut total_result = OutputResult {
                    output_paths: Vec::new(),
                    records_written: 0,
                    bytes_written: 0,
                    generation_time_ms: 0,
                    metadata: HashMap::new(),
                };

                let base_path = path.parent().unwrap_or(Path::new("."));
                let base_name = path.file_stem().unwrap_or(std::ffi::OsStr::new("output"));

                if !series.is_empty() {
                    let series_path = base_path.join(format!("{}_series.csv", base_name.to_string_lossy()));
                    let result = self.write_series(&series, &series_path, config).await?;
                    total_result.output_paths.extend(result.output_paths);
                    total_result.records_written += result.records_written;
                    total_result.bytes_written += result.bytes_written;
                    total_result.generation_time_ms += result.generation_time_ms;
                }

                if !observations.is_empty() {
                    let obs_path = base_path.join(format!("{}_observations.csv", base_name.to_string_lossy()));
                    let result = self.write_observations(&observations, &obs_path, config).await?;
                    total_result.output_paths.extend(result.output_paths);
                    total_result.records_written += result.records_written;
                    total_result.bytes_written += result.bytes_written;
                    total_result.generation_time_ms += result.generation_time_ms;
                }

                if !lookups.is_empty() {
                    let lookup_path = base_path.join(format!("{}_lookups.csv", base_name.to_string_lossy()));
                    let result = self.write_lookups(&lookups, &lookup_path, config).await?;
                    total_result.output_paths.extend(result.output_paths);
                    total_result.records_written += result.records_written;
                    total_result.bytes_written += result.bytes_written;
                    total_result.generation_time_ms += result.generation_time_ms;
                }

                if !surveys.is_empty() {
                    for (i, survey) in surveys.iter().enumerate() {
                        let survey_path = base_path.join(format!("{}_survey_{}.csv", base_name.to_string_lossy(), i));
                        let result = self.write_survey(survey, &survey_path, config).await?;
                        total_result.output_paths.extend(result.output_paths);
                        total_result.records_written += result.records_written;
                        total_result.bytes_written += result.bytes_written;
                        total_result.generation_time_ms += result.generation_time_ms;
                    }
                }

                Ok(total_result)
            }
        }
    }

    fn validate_format_config(&self, _config: &OutputConfig) -> Result<()> {
        // CSV format has minimal configuration requirements
        Ok(())
    }

    fn default_format_options(&self) -> HashMap<String, String> {
        let mut options = HashMap::new();
        options.insert("delimiter".to_string(), ",".to_string());
        options.insert("quote_char".to_string(), "\"".to_string());
        options.insert("escape_char".to_string(), "\"".to_string());
        options.insert("header".to_string(), "true".to_string());
        options
    }
}

/// CSV output generator that implements the OutputGenerator trait
pub struct CsvOutputGenerator {
    writer: CsvWriter,
}

impl CsvOutputGenerator {
    /// Create a new CSV output generator
    pub fn new() -> Self {
        Self {
            writer: CsvWriter::new(),
        }
    }
}

impl Default for CsvOutputGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutputGenerator for CsvOutputGenerator {
    fn name(&self) -> &str {
        "csv_generator"
    }

    fn description(&self) -> &str {
        "CSV output generator for BLS data"
    }

    fn supported_formats(&self) -> Vec<String> {
        vec!["csv".to_string()]
    }

    async fn generate(&mut self, data: ProcessedData, config: OutputConfig) -> Result<OutputResult> {
        let path = Path::new(&config.destination);
        self.writer.write_mixed(data, path, &config).await
    }

    fn validate_config(&self, config: &OutputConfig) -> Result<()> {
        if config.format.to_lowercase() != "csv" {
            return Err(ProcessingError::invalid_configuration(
                format!("CSV generator does not support format: {}", config.format)
            ));
        }
        self.writer.validate_format_config(config)
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
    use tempfile::TempDir;

    #[test]
    fn test_csv_writer_creation() {
        let writer = CsvWriter::new();
        assert_eq!(writer.format_name(), "csv");
        assert_eq!(writer.file_extension(), "csv");
    }

    #[test]
    fn test_csv_headers() {
        assert!(CsvWriter::series_header().contains("series_id"));
        assert!(CsvWriter::observation_header().contains("year"));
        assert!(CsvWriter::lookup_header().contains("code"));
        assert!(CsvWriter::survey_header().contains("survey_abbreviation"));
    }

    #[test]
    fn test_format_series_row() {
        let series = Series {
            series_id: "TEST001".to_string(),
            title: Some("Test Series".to_string()),
            area_code: Some("US".to_string()),
            item_code: Some("ITEM1".to_string()),
            seasonal: Some("S".to_string()),
            periodicity_code: Some("M".to_string()),
            base_code: Some("BASE".to_string()),
            base_period: "2020".to_string(),
        };

        let row = CsvWriter::format_series_row(&series);
        assert!(row.contains("TEST001"));
        assert!(row.contains("Test Series"));
        assert!(row.contains("US"));
    }

    #[test]
    fn test_csv_generator_creation() {
        let generator = CsvOutputGenerator::new();
        assert_eq!(generator.name(), "csv_generator");
        assert!(generator.supported_formats().contains(&"csv".to_string()));
    }

    #[test]
    fn test_csv_generator_validation() {
        let generator = CsvOutputGenerator::new();
        
        let valid_config = OutputConfig {
            format: "csv".to_string(),
            destination: "output.csv".to_string(),
            ..Default::default()
        };
        assert!(generator.validate_config(&valid_config).is_ok());

        let invalid_config = OutputConfig {
            format: "json".to_string(),
            destination: "output.json".to_string(),
            ..Default::default()
        };
        assert!(generator.validate_config(&invalid_config).is_err());
    }

    #[test]
    fn test_default_format_options() {
        let writer = CsvWriter::new();
        let options = writer.default_format_options();
        
        assert_eq!(options.get("delimiter"), Some(&",".to_string()));
        assert_eq!(options.get("header"), Some(&"true".to_string()));
    }
}