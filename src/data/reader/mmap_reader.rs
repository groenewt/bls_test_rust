//! Memory-mapped reader implementation for BLS data processing
//!
//! This module provides a high-performance reader implementation using memory mapping
//! for efficient processing of large BLS data files. It's optimized for files larger
//! than 1GB and provides zero-copy access to file data.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::sync::Arc;
use async_trait::async_trait;
use memmap2::{Mmap, MmapOptions};

use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::data::reader::traits::{
    DataReader, SeriesReader, ObservationReader, LookupReader, SurveyReader,
    MemoryMappedReader, StreamingReader, ReaderConfig, ReadStats,
};
use crate::error::types::{DataError, Result};
use crate::utils::validation::BLSValidationRules;

/// Memory-mapped reader implementation for large files
pub struct MmapReader {
    config: ReaderConfig,
    stats: ReadStats,
    current_file: Option<PathBuf>,
    file_handle: Option<File>,
    memory_map: Option<Arc<Mmap>>,
    current_position: usize,
    validation_rules: BLSValidationRules,
}

impl MmapReader {
    /// Create a new memory-mapped reader with the given configuration
    pub fn new(config: ReaderConfig) -> Self {
        Self {
            config,
            stats: ReadStats::default(),
            current_file: None,
            file_handle: None,
            memory_map: None,
            current_position: 0,
            validation_rules: BLSValidationRules::default(),
        }
    }

    /// Get the current line from the memory map starting at the given position
    fn get_line_at_position(&self, start_pos: usize) -> Option<(String, usize)> {
        if let Some(ref mmap) = self.memory_map {
            let data = mmap.as_ref();
            if start_pos >= data.len() {
                return None;
            }

            // Find the end of the line
            let mut end_pos = start_pos;
            while end_pos < data.len() && data[end_pos] != b'\n' && data[end_pos] != b'\r' {
                end_pos += 1;
            }

            // Skip over line ending characters
            let mut next_pos = end_pos;
            while next_pos < data.len() && (data[next_pos] == b'\n' || data[next_pos] == b'\r') {
                next_pos += 1;
            }

            // Convert bytes to string
            match std::str::from_utf8(&data[start_pos..end_pos]) {
                Ok(line) => Some((line.to_string(), next_pos)),
                Err(_) => {
                    // Try to handle encoding issues gracefully
                    let line = String::from_utf8_lossy(&data[start_pos..end_pos]).to_string();
                    Some((line, next_pos))
                }
            }
        } else {
            None
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
            ).into());
        }

        let series_id = fields[0].clone();
        let title = fields.get(1).unwrap_or(&String::new()).clone();
        let area_code = fields.get(2).unwrap_or(&String::new()).clone();

        Ok(Series::new(series_id, title, area_code))
    }

    /// Parse an observation record from fields
    fn parse_observation_record(&self, fields: &[String]) -> Result<Observation> {
        if fields.len() < 4 {
            return Err(DataError::invalid_format(
                "Observation record must have at least 4 fields".to_string(),
            ).into());
        }

        let series_id = fields[0].clone();
        let year = fields[1].parse::<i32>()
            .map_err(|e| DataError::parse_error(format!("Invalid year: {}", e)))?;
        let period = fields[2].clone();
        let value_str = &fields[3];

        let value = if value_str.is_empty() || value_str == "-" {
            None
        } else {
            Some(value_str.parse::<f64>()
                .map_err(|e| DataError::parse_error(format!("Invalid value: {}", e)))?)
        };

        Ok(Observation::new(series_id, year, period, value))
    }

    /// Parse a lookup record from fields
    fn parse_lookup_record(&self, fields: &[String]) -> Result<Lookup> {
        if fields.len() < 2 {
            return Err(DataError::invalid_format(
                "Lookup record must have at least 2 fields".to_string(),
            ).into());
        }

        let code = fields[0].clone();
        let name = fields[1].clone();
        let description = fields.get(2).cloned();

        Ok(Lookup::new(code, name, description))
    }

    /// Update statistics after processing records
    fn update_stats(&mut self, records_processed: u64, bytes_processed: u64, errors: u64, start_time: Instant) {
        self.stats.records_read += records_processed;
        self.stats.bytes_processed += bytes_processed;
        self.stats.errors_encountered += errors;
        self.stats.read_time_ms += start_time.elapsed().as_millis() as u64;
    }

    /// Count total lines in the memory-mapped file
    fn count_lines(&self) -> u64 {
        if let Some(ref mmap) = self.memory_map {
            let data = mmap.as_ref();
            data.iter().filter(|&&b| b == b'\n').count() as u64
        } else {
            0
        }
    }

    /// Find all line positions in the memory-mapped file
    fn find_line_positions(&self) -> Vec<usize> {
        let mut positions = vec![0]; // Start of file
        
        if let Some(ref mmap) = self.memory_map {
            let data = mmap.as_ref();
            for (i, &byte) in data.iter().enumerate() {
                if byte == b'\n' && i + 1 < data.len() {
                    positions.push(i + 1);
                }
            }
        }
        
        positions
    }
}

#[async_trait]
impl DataReader for MmapReader {
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

        // Check file size - memory mapping is most beneficial for larger files
        if let Ok(metadata) = path.metadata() {
            let file_size = metadata.len();
            // Only use memory mapping for files larger than 10MB
            if file_size < 10 * 1024 * 1024 {
                return Ok(false);
            }
        }

        // Check file extension or naming convention
        if let Some(extension) = path.extension() {
            match extension.to_str() {
                Some("series") | Some("data") | Some("area") | Some("item") | Some("txt") => Ok(true),
                _ => Ok(false),
            }
        } else {
            // Check if filename matches BLS naming patterns
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                Ok(filename.contains(".series") || 
                   filename.contains(".data") || 
                   filename.contains(".area") || 
                   filename.contains(".item"))
            } else {
                Ok(false)
            }
        }
    }

    async fn open(&mut self, path: &Path) -> Result<()> {
        if !self.can_read(path)? {
            return Err(DataError::unsupported_format(
                format!("Cannot read file with memory mapping: {}", path.display())
            ).into());
        }

        let file = File::open(path)
            .map_err(|e| DataError::IoError(format!("Failed to open file: {}", e)))?;

        // Create memory map
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .map_err(|e| DataError::IoError(format!("Failed to create memory map: {}", e)))?
        };

        self.file_handle = Some(file);
        self.memory_map = Some(Arc::new(mmap));
        self.current_file = Some(path.to_path_buf());
        self.current_position = 0;
        self.reset_stats();

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.memory_map = None;
        self.file_handle = None;
        self.current_file = None;
        self.current_position = 0;
        Ok(())
    }

    fn is_open(&self) -> bool {
        self.memory_map.is_some()
    }

    fn current_file(&self) -> Option<&Path> {
        self.current_file.as_deref()
    }
}

impl MemoryMappedReader for MmapReader {
    fn memory_map(&self) -> Result<&[u8]> {
        if let Some(ref mmap) = self.memory_map {
            Ok(mmap.as_ref())
        } else {
            Err(DataError::IoError("No memory map available".to_string()).into())
        }
    }

    fn slice(&self, start: usize, len: usize) -> Result<&[u8]> {
        if let Some(ref mmap) = self.memory_map {
            let data = mmap.as_ref();
            if start + len <= data.len() {
                Ok(&data[start..start + len])
            } else {
                Err(DataError::invalid_range(
                    format!("Slice range {}..{} exceeds file size {}", start, start + len, data.len())
                ).into())
            }
        } else {
            Err(DataError::IoError("No memory map available".to_string()).into())
        }
    }

    fn find_pattern(&self, pattern: &[u8]) -> Result<Vec<usize>> {
        if let Some(ref mmap) = self.memory_map {
            let data = mmap.as_ref();
            let mut positions = Vec::new();
            
            if pattern.is_empty() {
                return Ok(positions);
            }

            for i in 0..=data.len().saturating_sub(pattern.len()) {
                if data[i..i + pattern.len()] == *pattern {
                    positions.push(i);
                }
            }

            Ok(positions)
        } else {
            Err(DataError::IoError("No memory map available".to_string()).into())
        }
    }
}

#[async_trait]
impl SeriesReader for MmapReader {
    async fn read_all_series(&mut self) -> Result<Vec<Series>> {
        let start_time = Instant::now();
        let mut series_list = Vec::new();
        let mut errors = 0;
        let mut bytes_processed = 0;

        if self.memory_map.is_some() {
            let mut position = 0;

            while let Some((line, next_pos)) = self.get_line_at_position(position) {
                bytes_processed += (next_pos - position) as u64;
                position = next_pos;

                if line.trim().is_empty() {
                    continue;
                }

                let fields = self.parse_line(&line);

                if self.validate_record(&fields, "series").unwrap_or(false) {
                    match self.parse_series_record(&fields) {
                        Ok(series) => series_list.push(series),
                        Err(_) => {
                            errors += 1;
                            if !self.config.skip_malformed {
                                return Err(DataError::parse_error(
                                    format!("Failed to parse series record: {}", line)
                                ).into());
                            }
                        }
                    }
                } else {
                    errors += 1;
                    if !self.config.skip_malformed {
                        return Err(DataError::ValidationError(
                            format!("Invalid series record: {}", line)
                        ).into());
                    }
                }

                if errors > self.config.max_errors {
                    return Err(DataError::too_many_errors(
                        format!("Exceeded maximum error count: {}", self.config.max_errors)
                    ).into());
                }
            }
        } else {
            return Err(DataError::IoError("No memory map available".to_string()).into());
        }

        self.update_stats(series_list.len() as u64, bytes_processed, errors, start_time);
        Ok(series_list)
    }

    async fn read_series_batch(&mut self, batch_size: usize) -> Result<Vec<Series>> {
        let start_time = Instant::now();
        let mut series_list = Vec::new();
        let mut errors = 0;
        let mut bytes_processed = 0;

        if self.memory_map.is_some() {
            while series_list.len() < batch_size {
                if let Some((line, next_pos)) = self.get_line_at_position(self.current_position) {
                    bytes_processed += (next_pos - self.current_position) as u64;
                    self.current_position = next_pos;

                    if line.trim().is_empty() {
                        continue;
                    }

                    let fields = self.parse_line(&line);

                    if self.validate_record(&fields, "series").unwrap_or(false) {
                        match self.parse_series_record(&fields) {
                            Ok(series) => series_list.push(series),
                            Err(_) => {
                                errors += 1;
                                if !self.config.skip_malformed {
                                    return Err(DataError::parse_error(
                                        format!("Failed to parse series record: {}", line)
                                    ).into());
                                }
                            }
                        }
                    } else {
                        errors += 1;
                        if !self.config.skip_malformed {
                            return Err(DataError::ValidationError(
                                format!("Invalid series record: {}", line)
                            ).into());
                        }
                    }

                    if errors > self.config.max_errors {
                        return Err(DataError::too_many_errors(
                            format!("Exceeded maximum error count: {}", self.config.max_errors)
                        ).into());
                    }
                } else {
                    break; // EOF
                }
            }
        } else {
            return Err(DataError::IoError("No memory map available".to_string()).into());
        }

        self.update_stats(series_list.len() as u64, bytes_processed, errors, start_time);
        Ok(series_list)
    }

    async fn read_series_by_id(&mut self, series_id: &str) -> Result<Option<Series>> {
        let start_time = Instant::now();
        let mut bytes_processed = 0;
        let mut errors = 0;

        if self.memory_map.is_some() {
            let mut position = 0;

            while let Some((line, next_pos)) = self.get_line_at_position(position) {
                bytes_processed += (next_pos - position) as u64;
                position = next_pos;

                if line.trim().is_empty() {
                    continue;
                }

                let fields = self.parse_line(&line);

                if !fields.is_empty() && fields[0] == series_id {
                    if self.validate_record(&fields, "series").unwrap_or(false) {
                        match self.parse_series_record(&fields) {
                            Ok(series) => {
                                self.update_stats(1, bytes_processed, errors, start_time);
                                return Ok(Some(series));
                            }
                            Err(_) => {
                                errors += 1;
                                if !self.config.skip_malformed {
                                    return Err(DataError::parse_error(
                                        format!("Failed to parse series record: {}", line)
                                    ).into());
                                }
                            }
                        }
                    }
                }
            }
        } else {
            return Err(DataError::IoError("No memory map available".to_string()).into());
        }

        self.update_stats(0, bytes_processed, errors, start_time);
        Ok(None)
    }

    async fn count_series(&mut self) -> Result<u64> {
        let start_time = Instant::now();
        let count = self.count_lines();
        let bytes_processed = if let Some(ref mmap) = self.memory_map {
            mmap.len() as u64
        } else {
            0
        };

        self.update_stats(count, bytes_processed, 0, start_time);
        Ok(count)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_mmap_reader_creation() {
        let config = ReaderConfig::default();
        let reader = MmapReader::new(config);
        assert!(!reader.is_open());
        assert!(reader.current_file().is_none());
    }

    #[tokio::test]
    async fn test_can_read_large_file() {
        // Create a temporary file larger than 10MB
        let mut temp_file = NamedTempFile::with_suffix(".series").unwrap();
        let data = "test data\n".repeat(1_000_000); // ~10MB
        temp_file.write_all(data.as_bytes()).unwrap();
        
        let config = ReaderConfig::default();
        let reader = MmapReader::new(config);
        
        assert!(reader.can_read(temp_file.path()).unwrap());
    }

    #[tokio::test]
    async fn test_memory_map_access() {
        let mut temp_file = NamedTempFile::with_suffix(".series").unwrap();
        let data = "test data\n".repeat(1_000_000);
        temp_file.write_all(data.as_bytes()).unwrap();
        
        let config = ReaderConfig::default();
        let mut reader = MmapReader::new(config);
        
        reader.open(temp_file.path()).await.unwrap();
        
        let mmap_data = reader.memory_map().unwrap();
        assert!(!mmap_data.is_empty());
        
        let slice = reader.slice(0, 9).unwrap();
        assert_eq!(slice, b"test data");
    }

    #[tokio::test]
    async fn test_find_pattern() {
        let mut temp_file = NamedTempFile::with_suffix(".series").unwrap();
        let data = "SERIES001\tTest Series\tAREA001\nSERIES002\tAnother Series\tAREA002\n";
        temp_file.write_all(data.as_bytes()).unwrap();
        
        let config = ReaderConfig::default();
        let mut reader = MmapReader::new(config);
        
        reader.open(temp_file.path()).await.unwrap();
        
        let positions = reader.find_pattern(b"SERIES").unwrap();
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], 0);
        assert!(positions[1] > 0);
    }
}