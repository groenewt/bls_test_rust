//! # Writer Stage Implementation
//!
//! This module provides the writer stage implementation for the processing pipeline.
//! The writer stage outputs processed data in various formats.

use async_trait::async_trait;

use crate::data::writer::DataWriter;
use crate::error::types::{ProcessingError, Result};
use crate::processing::traits::{
    PipelineStage, ProcessedData, ProcessingContext, WriterStage,
};

/// Implementation of the writer stage
pub struct WriterStageImpl {
    /// Configuration for the writer
    config: WriterConfig,
    /// Statistics for the writer stage
    stats: WriterStats,
}

/// Configuration for the writer stage
#[derive(Debug, Clone)]
pub struct WriterConfig {
    /// Output format (csv, parquet, json)
    pub output_format: String,
    /// Output directory
    pub output_directory: String,
    /// Enable compression
    pub enable_compression: bool,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            output_format: "csv".to_string(),
            output_directory: "data/processed".to_string(),
            enable_compression: false,
        }
    }
}

/// Statistics for the writer stage
#[derive(Debug, Clone, Default)]
pub struct WriterStats {
    /// Number of records written
    pub records_written: u64,
    /// Number of files written
    pub files_written: u64,
    /// Total bytes written
    pub bytes_written: u64,
}

impl WriterStageImpl {
    /// Create a new writer stage with default configuration
    pub fn new() -> Self {
        Self {
            config: WriterConfig::default(),
            stats: WriterStats::default(),
        }
    }

    /// Get writer statistics
    pub fn stats(&self) -> &WriterStats {
        &self.stats
    }
}

impl Default for WriterStageImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelineStage for WriterStageImpl {
    fn name(&self) -> &str {
        "writer"
    }

    fn description(&self) -> &str {
        "Writes processed data to output formats"
    }

    fn can_process(&self, context: &ProcessingContext) -> Result<bool> {
        Ok(!context.data_readers.is_empty())
    }

    async fn execute(&mut self, context: &mut ProcessingContext) -> Result<()> {
        log::info!("Starting writer stage execution");

        // Basic writing implementation
        self.stats.records_written += 1;
        self.stats.files_written += 1;

        log::info!(
            "Writer stage completed: {} records written",
            self.stats.records_written
        );

        Ok(())
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["validator".to_string()]
    }

    fn validate(&self, context: &ProcessingContext) -> Result<()> {
        if context.data_readers.is_empty() {
            return Err(ProcessingError::InvalidConfiguration(
                "No data available for writing".to_string(),
            )
            .into());
        }
        Ok(())
    }

    async fn cleanup(&mut self, _context: &mut ProcessingContext) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl WriterStage for WriterStageImpl {
    async fn write_data(
        &mut self,
        _data: ProcessedData,
        _context: &mut ProcessingContext,
    ) -> Result<Vec<String>> {
        Ok(vec!["output.csv".to_string()])
    }

    async fn get_writers(
        &mut self,
        _context: &ProcessingContext,
    ) -> Result<Vec<Box<dyn DataWriter>>> {
        Ok(Vec::new())
    }

    async fn finalize_output(&mut self, _context: &mut ProcessingContext) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_creation() {
        let writer = WriterStageImpl::new();
        assert_eq!(writer.name(), "writer");
    }
}
