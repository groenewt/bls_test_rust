//! File-based reader implementation for BLS data processing
//!
//! This module provides concrete implementations of the reader traits using
//! standard file I/O operations. It's optimized for small to medium-sized files
//! and provides robust error handling and validation.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::fs::File as AsyncFile;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};

use crate::data::model::{Lookup, Observation, Series};
use crate::data::reader::traits::{
    DataReader, LookupIterator, LookupReader, ObservationIterator, ObservationReader, ReadStats,
    ReaderConfig, SeriesIterator, SeriesReader,
};
use crate::error::types::{DataError, Result};
use crate::utils::validation::BLSValidationRules;

/// File-based reader implementation
#[derive(Debug)]
pub struct FileReader {
    config: ReaderConfig,
    stats: ReadStats,
    current_file: Option<PathBuf>,
    file_handle: Option<AsyncFile>,
    validation_rules: BLSValidationRules,
}

impl FileReader {
    /// Create a new file reader with the given configuration
    pub fn new(config: ReaderConfig) -> Self {
        Self {
            config,
            stats: ReadStats::default(),
            current_file: None,
            file_handle: None,
            validation_rules: BLSValidationRules::default(),
        }
    }

    /// Parse a line into fields based on the configured separator
    fn parse_line(&self, line: &str) -> Vec<String> {
        line.split(self.config.field_separator)
            .map(|s| s.trim().to_string())
            .collect()
    }

    /// Validate a record based on the configured validation rules
    fn validate_record(&self, fields: &[String], record_type: &str) -> Result<bool> {
        if !self.config.validate_on_read {
            return Ok(true);
        }

        match record_type {
            "series" => self.validation_rules.validate_series_record(fields),
            "observation" => self.validation_rules.validate_observation_record(fields),
            "lookup" => self.validation_rules.validate_lookup_record(fields),
            _ => Ok(true),
        }
    }

    /// Parse a series record from fields
    fn parse_series_record(&self, fields: &[String]) -> Result<Series> {
        if fields.len() < 3 {
            return Err(DataError::invalid_format(
                "Series record must have at least 3 fields".to_string(),
            )
            .into());
        }

        // Basic series parsing - this would be expanded based on actual BLS format
        let series_id = fields[0].clone();
        let title = fields.get(1).unwrap_or(&String::new()).clone();

        Ok(Series::new(&series_id, &title))
    }

    /// Parse an observation record from fields
    fn parse_observation_record(&self, fields: &[String]) -> Result<Observation> {
        if fields.len() < 4 {
            return Err(DataError::invalid_format(
                "Observation record must have at least 4 fields".to_string(),
            )
            .into());
        }

        let series_id = fields[0].clone();
        let year = fields[1]
            .parse::<i32>()
            .map_err(|e| DataError::parse_error(format!("Invalid year: {e}")))?;
        let period = fields[2].clone();
        let value_str = &fields[3];

        let value = if value_str.is_empty() || value_str == "-" {
            None
        } else {
            Some(
                value_str
                    .parse::<f64>()
                    .map_err(|e| DataError::parse_error(format!("Invalid value: {e}")))?,
            )
        };

        Ok(Observation::new(&series_id, &year, &period, value))
    }

    /// Parse a lookup record from fields
    fn parse_lookup_record(&self, fields: &[String]) -> Result<Lookup> {
        if fields.len() < 2 {
            return Err(DataError::invalid_format(
                "Lookup record must have at least 2 fields".to_string(),
            )
            .into());
        }

        let code = fields[0].clone();
        let name = fields[1].clone();
        let _description = fields.get(2).cloned();

        Ok(Lookup::new(&code, &name))
    }

    /// Update statistics after processing records
    fn update_stats(
        &mut self,
        records_processed: u64,
        bytes_processed: u64,
        errors: u64,
        start_time: Instant,
    ) {
        self.stats.records_read += records_processed;
        self.stats.bytes_processed += bytes_processed;
        self.stats.errors_encountered += errors;
        self.stats.read_time_ms += start_time.elapsed().as_millis() as u64;
    }

    /// Static version of parse_observation_record that doesn't borrow self
    fn parse_observation_record_static(fields: &[String]) -> Result<Observation> {
        if fields.len() < 4 {
            return Err(DataError::invalid_format(
                "Observation record must have at least 4 fields".to_string(),
            )
            .into());
        }

        let series_id = fields[0].clone();
        let year = fields[1]
            .parse::<i32>()
            .map_err(|e| DataError::parse_error(format!("Invalid year: {e}")))?;
        let period = fields[2].clone();
        let value_str = &fields[3];

        let value = if value_str.is_empty() || value_str == "-" {
            None
        } else {
            Some(
                value_str
                    .parse::<f64>()
                    .map_err(|e| DataError::parse_error(format!("Invalid value: {e}")))?,
            )
        };

        Ok(Observation::new(&series_id, &year, &period, value))
    }

    /// Static version of parse_lookup_record that doesn't borrow self
    fn parse_lookup_record_static(fields: &[String]) -> Result<Lookup> {
        if fields.len() < 2 {
            return Err(DataError::invalid_format(
                "Lookup record must have at least 2 fields".to_string(),
            )
            .into());
        }

        let code = fields[0].clone();
        let name = fields[1].clone();

        Ok(Lookup::new(&code, &name))
    }

    /// Static version of parse_series_record that doesn't borrow self
    fn parse_series_record_static(fields: &[String]) -> Result<Series> {
        if fields.len() < 4 {
            return Err(DataError::invalid_format(
                "Series record must have at least 4 fields".to_string(),
            )
            .into());
        }

        let series_id = fields[0].clone();
        let title = fields.get(1).cloned();
        let area_code = fields.get(2).cloned();
        let item_code = fields.get(3).cloned();
        let survey_code = series_id.chars().take(2).collect::<String>();

        Ok(Series {
            series_id,
            title: title.unwrap_or_default(),
            area_code: area_code.unwrap_or_default(),
            item_code: item_code.unwrap_or_default(),
            survey_code,
            metadata: crate::data::model::series::SeriesMetadata::default(),
            common: crate::data::model::CommonMetadata::new(),
            periodicity_code: String::new(),
            seasonal: String::new(),
            base_period: String::new(),
            base_code: String::new(),
        })
    }
}

#[async_trait]
impl DataReader for FileReader {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn config(&self) -> &ReaderConfig {
        &self.config
    }

    fn stats(&self) -> &ReadStats {
        &self.stats
    }

    fn reset_stats(&mut self) {
        self.stats = ReadStats::default();
    }

    fn can_read(&self, path: &Path) -> Result<bool> {
        if !path.exists() {
            return Ok(false);
        }

        if !path.is_file() {
            return Ok(false);
        }

        // Check file extension or naming convention
        if let Some(extension) = path.extension() {
            match extension.to_str() {
                Some("series") | Some("data") | Some("area") | Some("item") | Some("txt") => {
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            // Check if filename matches BLS naming patterns
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                Ok(filename.contains(".series")
                    || filename.contains(".data")
                    || filename.contains(".area")
                    || filename.contains(".item"))
            } else {
                Ok(false)
            }
        }
    }

    async fn open(&mut self, path: &Path) -> Result<()> {
        if !self.can_read(path)? {
            return Err(DataError::unsupported_format(format!(
                "Cannot read file: {}",
                path.display()
            ))
            .into());
        }

        let file = AsyncFile::open(path)
            .await
            .map_err(|e| DataError::io_error(format!("Failed to open file: {e}")))?;

        self.file_handle = Some(file);
        self.current_file = Some(path.to_path_buf());
        self.reset_stats();

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.file_handle = None;
        self.current_file = None;
        Ok(())
    }

    fn is_open(&self) -> bool {
        self.file_handle.is_some()
    }

    fn current_file(&self) -> Option<&Path> {
        self.current_file.as_deref()
    }
}

#[async_trait]
impl SeriesReader for FileReader {
    async fn read_all_series(&mut self) -> Result<Vec<Series>> {
        let start_time = Instant::now();
        let mut series_list = Vec::new();
        let mut errors = 0u64;
        let mut bytes_processed = 0;

        // Extract config values before the borrow
        let max_errors = self.config.max_errors as u64;
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::io_error(format!("Failed to read line: {e}")))?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 4 {
                    match Self::parse_series_record_static(&fields) {
                        Ok(series) => series_list.push(series),
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::parse_error(format!(
                                    "Failed to parse series record: {line}"
                                ))
                                .into());
                            }
                        }
                    }
                } else {
                    errors += 1;
                    if !skip_malformed {
                        return Err(DataError::missing_value(format!(
                            "Invalid series record: {line}"
                        ))
                        .into());
                    }
                }

                if errors > max_errors {
                    return Err(DataError::too_many_errors(format!(
                        "Exceeded maximum error count: {max_errors}"
                    ))
                    .into());
                }

                line.clear();
            }
        } else {
            return Err(DataError::io_error("No file is currently open".to_string()).into());
        }

        self.update_stats(
            series_list.len() as u64,
            bytes_processed,
            errors,
            start_time,
        );
        Ok(series_list)
    }

    async fn read_series_batch(&mut self, batch_size: usize) -> Result<Vec<Series>> {
        let start_time = Instant::now();
        let mut series_list = Vec::new();
        let mut errors = 0u64;
        let mut bytes_processed = 0;

        // Extract config values before the borrow
        let max_errors = self.config.max_errors as u64;
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while series_list.len() < batch_size {
                let bytes_read = reader
                    .read_line(&mut line)
                    .await
                    .map_err(|e| DataError::io_error(format!("Failed to read line: {e}")))?;

                if bytes_read == 0 {
                    break; // EOF
                }

                bytes_processed += bytes_read as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 4 {
                    match Self::parse_series_record_static(&fields) {
                        Ok(series) => series_list.push(series),
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::parse_error(format!(
                                    "Failed to parse series record: {line}"
                                ))
                                .into());
                            }
                        }
                    }
                } else {
                    errors += 1;
                    if !skip_malformed {
                        return Err(DataError::ValidationError {
                            message: format!("Invalid series record: {line}"),
                            path: None,
                            line: None,
                        }
                        .into());
                    }
                }

                if errors > max_errors {
                    return Err(DataError::too_many_errors(format!(
                        "Exceeded maximum error count: {max_errors}"
                    ))
                    .into());
                }

                line.clear();
            }
        } else {
            return Err(DataError::io_error("No file is currently open".to_string()).into());
        }

        self.update_stats(
            series_list.len() as u64,
            bytes_processed,
            errors,
            start_time,
        );
        Ok(series_list)
    }

    async fn read_series_by_id(&mut self, series_id: &str) -> Result<Option<Series>> {
        let start_time = Instant::now();
        let mut bytes_processed = 0;
        let mut errors = 0u64;

        // Extract config values before the borrow
        let field_separator = self.config.field_separator;

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::io_error(format!("Failed to read line: {e}")))?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if !fields.is_empty() && fields[0] == series_id {
                    // Simple validation without borrowing self
                    if fields.len() >= 4 {
                        match Self::parse_series_record_static(&fields) {
                            Ok(series) => {
                                self.update_stats(1, bytes_processed, errors, start_time);
                                return Ok(Some(series));
                            }
                            Err(_) => {
                                errors += 1;
                                if !self.config.skip_malformed {
                                    return Err(DataError::parse_error(format!(
                                        "Failed to parse series record: {line}"
                                    ))
                                    .into());
                                }
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::io_error("No file is currently open".to_string()).into());
        }

        self.update_stats(0, bytes_processed, errors, start_time);
        Ok(None)
    }

    async fn count_series(&mut self) -> Result<u64> {
        let start_time = Instant::now();
        let mut count = 0;
        let mut bytes_processed = 0;

        // Extract config values before the borrow
        let field_separator = self.config.field_separator;

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::io_error(format!("Failed to read line: {e}")))?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Simple validation without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();
                if fields.len() >= 4 {
                    count += 1;
                }

                line.clear();
            }
        } else {
            return Err(DataError::io_error("No file is currently open".to_string()).into());
        }

        self.update_stats(count, bytes_processed, 0, start_time);
        Ok(count)
    }
}

#[async_trait]
impl SeriesIterator for FileReader {
    async fn for_each_series<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(Series) -> Result<()> + Send + Sync,
    {
        let start_time = Instant::now();
        let mut processed = 0;
        let mut bytes_processed = 0;
        let mut errors = 0u64;

        // Extract config values before the borrow
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::io_error(format!("Failed to read line: {e}")))?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 4 {
                    match Self::parse_series_record_static(&fields) {
                        Ok(series) => {
                            callback(series)?;
                            processed += 1;
                        }
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::parse_error(format!(
                                    "Failed to parse series record: {line}"
                                ))
                                .into());
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::io_error("No file is currently open".to_string()).into());
        }

        self.update_stats(processed, bytes_processed, errors, start_time);
        Ok(())
    }
}

#[async_trait]
impl ObservationReader for FileReader {
    async fn read_all_observations(&mut self) -> Result<Vec<Observation>> {
        let start_time = Instant::now();
        let mut observations = Vec::new();
        let mut bytes_processed = 0;
        let mut errors = 0u64;

        // Extract config values before the borrow
        let max_errors = self.config.max_errors as u64;
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::ReadError {
                    path: current_file_name.clone(),
                    source: format!("Failed to read line: {e}"),
                })?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 4 {
                    match Self::parse_observation_record_static(&fields) {
                        Ok(observation) => observations.push(observation),
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::ValidationError {
                                    message: format!(
                                        "Failed to parse observation record: {}",
                                        line.trim()
                                    ),
                                    path: Some(current_file_name),
                                    line: None,
                                }
                                .into());
                            }
                        }
                    }
                }

                if errors > max_errors {
                    return Err(DataError::ValidationError {
                        message: format!("Exceeded maximum error count: {max_errors}"),
                        path: Some(current_file_name),
                        line: None,
                    }
                    .into());
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(
            observations.len() as u64,
            bytes_processed,
            errors,
            start_time,
        );
        Ok(observations)
    }

    async fn read_observations_batch(&mut self, batch_size: usize) -> Result<Vec<Observation>> {
        let start_time = Instant::now();
        let mut observations = Vec::with_capacity(batch_size);
        let mut bytes_processed = 0;
        let mut errors = 0u64;

        // Extract config values before the borrow
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while observations.len() < batch_size {
                let bytes_read =
                    reader
                        .read_line(&mut line)
                        .await
                        .map_err(|e| DataError::ReadError {
                            path: current_file_name.clone(),
                            source: format!("Failed to read line: {e}"),
                        })?;

                if bytes_read == 0 {
                    break; // End of file
                }

                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 4 {
                    match Self::parse_observation_record_static(&fields) {
                        Ok(observation) => observations.push(observation),
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::ValidationError {
                                    message: format!(
                                        "Failed to parse observation record: {}",
                                        line.trim()
                                    ),
                                    path: Some(current_file_name),
                                    line: None,
                                }
                                .into());
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(
            observations.len() as u64,
            bytes_processed,
            errors,
            start_time,
        );
        Ok(observations)
    }

    async fn read_observations_for_series(&mut self, series_id: &str) -> Result<Vec<Observation>> {
        let start_time = Instant::now();
        let mut observations = Vec::new();
        let mut bytes_processed = 0;
        let mut errors = 0u64;

        // Extract config values before the borrow
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::ReadError {
                    path: current_file_name.clone(),
                    source: format!("Failed to read line: {e}"),
                })?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 4 {
                    match Self::parse_observation_record_static(&fields) {
                        Ok(observation) => {
                            if observation.series_id() == series_id {
                                observations.push(observation);
                            }
                        }
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::ValidationError {
                                    message: format!(
                                        "Failed to parse observation record: {}",
                                        line.trim()
                                    ),
                                    path: Some(current_file_name),
                                    line: None,
                                }
                                .into());
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(
            observations.len() as u64,
            bytes_processed,
            errors,
            start_time,
        );
        Ok(observations)
    }

    async fn read_observations_by_date_range(
        &mut self,
        start_date: &str,
        end_date: &str,
    ) -> Result<Vec<Observation>> {
        let start_time = Instant::now();
        let mut observations = Vec::new();
        let mut bytes_processed = 0;
        let mut errors = 0u64;

        // Extract config values before the borrow
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::ReadError {
                    path: current_file_name.clone(),
                    source: format!("Failed to read line: {e}"),
                })?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 4 {
                    match Self::parse_observation_record_static(&fields) {
                        Ok(observation) => {
                            let obs_date = observation.period();
                            if obs_date >= start_date && obs_date <= end_date {
                                observations.push(observation);
                            }
                        }
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::ValidationError {
                                    message: format!(
                                        "Failed to parse observation record: {}",
                                        line.trim()
                                    ),
                                    path: Some(current_file_name),
                                    line: None,
                                }
                                .into());
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(
            observations.len() as u64,
            bytes_processed,
            errors,
            start_time,
        );
        Ok(observations)
    }

    async fn count_observations(&mut self) -> Result<u64> {
        let start_time = Instant::now();
        let mut count = 0;
        let mut bytes_processed = 0;

        // Extract config values before the borrow
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::ReadError {
                    path: current_file_name.clone(),
                    source: format!("Failed to read line: {e}"),
                })?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Simple validation without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();
                if fields.len() >= 4 {
                    count += 1;
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(count, bytes_processed, 0, start_time);
        Ok(count)
    }
}

// LookupReader implementation for FileReader
#[async_trait]
impl LookupReader for FileReader {
    async fn read_all_lookups(&mut self) -> Result<Vec<Lookup>> {
        let start_time = Instant::now();
        let mut lookups = Vec::new();
        let mut bytes_processed = 0;
        let mut errors = 0u64;

        // Extract config values before the borrow
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::ReadError {
                    path: current_file_name.clone(),
                    source: format!("Failed to read line: {e}"),
                })?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 2 {
                    match Self::parse_lookup_record_static(&fields) {
                        Ok(lookup) => lookups.push(lookup),
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::ValidationError {
                                    message: format!(
                                        "Failed to parse lookup record: {}",
                                        line.trim()
                                    ),
                                    path: Some(current_file_name),
                                    line: None,
                                }
                                .into());
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(lookups.len() as u64, bytes_processed, errors, start_time);
        Ok(lookups)
    }

    async fn read_lookups_batch(&mut self, batch_size: usize) -> Result<Vec<Lookup>> {
        let start_time = Instant::now();
        let mut lookups = Vec::with_capacity(batch_size);
        let mut bytes_processed = 0;
        let mut errors = 0u64;

        // Extract config values before the borrow
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while lookups.len() < batch_size {
                let bytes_read =
                    reader
                        .read_line(&mut line)
                        .await
                        .map_err(|e| DataError::ReadError {
                            path: current_file_name.clone(),
                            source: format!("Failed to read line: {e}"),
                        })?;

                if bytes_read == 0 {
                    break; // EOF
                }

                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 2 {
                    match Self::parse_lookup_record_static(&fields) {
                        Ok(lookup) => lookups.push(lookup),
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::ValidationError {
                                    message: format!(
                                        "Failed to parse lookup record: {}",
                                        line.trim()
                                    ),
                                    path: Some(current_file_name),
                                    line: None,
                                }
                                .into());
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(lookups.len() as u64, bytes_processed, errors, start_time);
        Ok(lookups)
    }

    async fn read_lookup_by_code(&mut self, code: &str) -> Result<Option<Lookup>> {
        let start_time = Instant::now();
        let mut bytes_processed = 0;
        let mut errors = 0u64;

        // Extract config values before the borrow
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::ReadError {
                    path: current_file_name.clone(),
                    source: format!("Failed to read line: {e}"),
                })?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 2 {
                    match Self::parse_lookup_record_static(&fields) {
                        Ok(lookup) => {
                            // Check if this lookup contains the requested code
                            if lookup.contains_entry(code) {
                                self.update_stats(1, bytes_processed, errors, start_time);
                                return Ok(Some(lookup));
                            }
                        }
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::ValidationError {
                                    message: format!(
                                        "Failed to parse lookup record: {}",
                                        line.trim()
                                    ),
                                    path: Some(current_file_name),
                                    line: None,
                                }
                                .into());
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(0, bytes_processed, errors, start_time);
        Ok(None)
    }

    async fn count_lookups(&mut self) -> Result<u64> {
        let start_time = Instant::now();
        let mut count = 0;
        let mut bytes_processed = 0;

        // Extract config values before the borrow
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::ReadError {
                    path: current_file_name.clone(),
                    source: format!("Failed to read line: {e}"),
                })?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Simple validation without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();
                if fields.len() >= 2 {
                    count += 1;
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(count, bytes_processed, 0, start_time);
        Ok(count)
    }
}

// ObservationIterator implementation for FileReader
#[async_trait]
impl ObservationIterator for FileReader {
    async fn for_each_observation<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(Observation) -> Result<()> + Send + Sync,
    {
        let start_time = Instant::now();
        let mut processed = 0;
        let mut bytes_processed = 0;
        let mut errors = 0;

        // Extract config values before the borrow
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::ReadError {
                    path: current_file_name.clone(),
                    source: format!("Failed to read line: {e}"),
                })?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 4 {
                    match Self::parse_observation_record_static(&fields) {
                        Ok(observation) => {
                            callback(observation)?;
                            processed += 1;
                        }
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::ValidationError {
                                    message: format!(
                                        "Failed to parse observation record: {}",
                                        line.trim()
                                    ),
                                    path: Some(current_file_name),
                                    line: None,
                                }
                                .into());
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(processed, bytes_processed, errors as u64, start_time);
        Ok(())
    }
}

// LookupIterator implementation for FileReader
#[async_trait]
impl LookupIterator for FileReader {
    async fn for_each_lookup<F>(&mut self, callback: F) -> Result<()>
    where
        F: Fn(Lookup) -> Result<()> + Send + Sync,
    {
        let start_time = Instant::now();
        let mut processed = 0;
        let mut bytes_processed = 0;
        let mut errors = 0;

        // Extract config values before the borrow
        let skip_malformed = self.config.skip_malformed;
        let field_separator = self.config.field_separator;
        let current_file_name = self
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        if let Some(ref mut file) = self.file_handle {
            let mut reader = AsyncBufReader::new(file);
            let mut line = String::new();

            while reader
                .read_line(&mut line)
                .await
                .map_err(|e| DataError::ReadError {
                    path: current_file_name.clone(),
                    source: format!("Failed to read line: {e}"),
                })?
                > 0
            {
                bytes_processed += line.len() as u64;

                // Parse line without borrowing self
                let fields: Vec<String> = line
                    .trim()
                    .split(field_separator)
                    .map(|s| s.to_string())
                    .collect();

                if fields.len() >= 2 {
                    match Self::parse_lookup_record_static(&fields) {
                        Ok(lookup) => {
                            callback(lookup)?;
                            processed += 1;
                        }
                        Err(_) => {
                            errors += 1;
                            if !skip_malformed {
                                return Err(DataError::ValidationError {
                                    message: format!(
                                        "Failed to parse lookup record: {}",
                                        line.trim()
                                    ),
                                    path: Some(current_file_name),
                                    line: None,
                                }
                                .into());
                            }
                        }
                    }
                }

                line.clear();
            }
        } else {
            return Err(DataError::ReadError {
                path: "unknown".to_string(),
                source: "No file is currently open".to_string(),
            }
            .into());
        }

        self.update_stats(processed, bytes_processed, errors as u64, start_time);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_file_reader_creation() {
        let config = ReaderConfig::default();
        let reader = FileReader::new(config);
        assert!(!reader.is_open());
        assert!(reader.current_file().is_none());
    }

    #[tokio::test]
    async fn test_can_read_valid_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test data").unwrap();

        let config = ReaderConfig::default();
        let reader = FileReader::new(config);

        // This would return false because the temp file doesn't have a BLS extension
        // In a real test, we'd create a file with the proper extension
        assert!(!reader.can_read(temp_file.path()).unwrap());
    }

    #[tokio::test]
    async fn test_parse_line() {
        let config = ReaderConfig::default();
        let reader = FileReader::new(config);

        let line = "field1\tfield2\tfield3";
        let fields = reader.parse_line(line);

        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "field1");
        assert_eq!(fields[1], "field2");
        assert_eq!(fields[2], "field3");
    }

    #[tokio::test]
    async fn test_parse_series_record() {
        let config = ReaderConfig::default();
        let reader = FileReader::new(config);

        let fields = vec!["SERIES001".to_string(), "Test Series".to_string()];

        let series = reader.parse_series_record(&fields).unwrap();
        assert_eq!(series.id(), "SERIES001");
    }
}
