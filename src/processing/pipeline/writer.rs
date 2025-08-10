//! # Writer Stage Implementation
//!
//! This module provides the writer stage implementation for the processing pipeline.
//! The writer stage outputs processed data in various formats.

use async_trait::async_trait;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::data::writer::{self, DataWriter, CsvDataWriter};
use crate::data::writer::traits as writer_traits;
use crate::data::writer::traits::{SeriesWriter, ObservationWriter, LookupWriter};
use crate::error::types::{ProcessingError, Result};
use crate::processing::traits::{
    PipelineStage, ProcessedData, ProcessingContext, WriterStage,
};

// For combined Parquet output
use arrow::array::{ArrayRef, Float64Builder, Int32Builder, StringBuilder};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;

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
        // Writer can proceed if we have any data to write or at least valid input paths.
        // Loader stage in this pipeline populates series/observation/lookup data directly
        // and does not retain DataReader instances in context.
        let has_data = !context.series_data.is_empty()
            || !context.observation_data.is_empty()
            || !context.lookup_data.is_empty();
        Ok(has_data || !context.input_paths.is_empty())
    }

    async fn execute(&mut self, context: &mut ProcessingContext) -> Result<()> {
        log::info!("Starting writer stage execution");
        // Determine survey code from input paths or series data
        let survey_code = infer_survey_code(context);
        let out_dir = format!("{}/{}", self.config.output_directory, survey_code);
        if let Err(e) = fs::create_dir_all(&out_dir) {
            return Err(ProcessingError::SystemError(format!(
                "Failed to create output directory {}: {}",
                out_dir, e
            ))
            .into());
        }

        // Helper to build writer config
        let mut mk_cfg = || {
            let mut cfg = writer_traits::WriterConfig::default();
            cfg.overwrite_existing = true;
            cfg.include_headers = true;
            // Ensure true CSV output: use comma as field separator for .csv files
            cfg.field_separator = ',';
            cfg
        };

        // Write series
        if !context.series_data.is_empty() {
            let path = Path::new(&out_dir).join("series.csv");
            let mut w = CsvDataWriter::new(mk_cfg());
            w.open(&path).await?;
            w.write_series_batch(&context.series_data).await?;
            w.close().await?;
            self.stats.files_written += 1;
            self.stats.records_written += w.stats().records_written;
        }

        // Write observations
        if !context.observation_data.is_empty() {
            let path = Path::new(&out_dir).join("observations.csv");
            let mut w = CsvDataWriter::new(mk_cfg());
            w.open(&path).await?;
            w.write_observations_batch(&context.observation_data).await?;
            w.close().await?;
            self.stats.files_written += 1;
            self.stats.records_written += w.stats().records_written;
        }

        // Write lookups
        if !context.lookup_data.is_empty() {
            let path = Path::new(&out_dir).join("lookups.csv");
            let mut w = CsvDataWriter::new(mk_cfg());
            w.open(&path).await?;
            w.write_lookups_batch(&context.lookup_data).await?;
            w.close().await?;
            self.stats.files_written += 1;
            self.stats.records_written += w.stats().records_written;
        }

        // Also produce combined Parquet under data/final/<SURVEY>/combined.parquet
        // This joins observations with series title and resolves lookup names where available
        // Always attempt to create the final Parquet, even if there are zero observations,
        // to ensure consistent downstream availability and integration.
        {
            let final_dir = format!("data/final/{}", survey_code);
            if let Err(e) = fs::create_dir_all(&final_dir) {
                return Err(ProcessingError::SystemError(format!(
                    "Failed to create final output directory {}: {}",
                    final_dir, e
                )).into());
            }

            // Build series map by series_id for fast joins
            let mut series_map: HashMap<String, &crate::data::model::Series> = HashMap::new();
            for s in &context.series_data {
                series_map.insert(s.series_id.clone(), s);
            }

            // Build lookup maps generically: table_id -> (code -> description)
            let mut lookup_maps: HashMap<String, HashMap<String, String>> = HashMap::new();
            for lk in &context.lookup_data {
                let tid = lk.table_id.to_lowercase();
                let table = lookup_maps.entry(tid).or_insert_with(HashMap::new);
                for (code, entry) in &lk.entries {
                    table.insert(code.clone(), entry.description.clone());
                }
            }

            // Preallocate builders (Arrow 56 builders don't take capacity)
            let _obs_len = context.observation_data.len();
            let mut b_series_id = StringBuilder::new();
            let mut b_series_title = StringBuilder::new();
            let mut b_survey_code = StringBuilder::new();
            let mut b_year = Int32Builder::new();
            let mut b_period_code = StringBuilder::new();
            let mut b_period_name = StringBuilder::new();
            let mut b_value = Float64Builder::new();

            // Determine dynamic lookup columns based on model.yml (preferred) and available lookups (fallback)
            let mut dyn_ids: Vec<String> = Vec::new();
            // Try to load lookup ids from YAML model config for this survey
            // Use config/surveys/<SC>/model.yml per guidelines
            let model_path = {
                let sc = survey_code.to_uppercase();
                Path::new("config").join("surveys").join(&sc).join("model.yml")
            };
            if model_path.exists() {
                match crate::config::yaml_adapter::load_yaml_lookup_ids(&model_path.to_path_buf()) {
                    Ok(mut ids) => {
                        // Preserve YAML order
                        dyn_ids.append(&mut ids);
                    }
                    Err(e) => {
                        log::warn!("Failed to read lookup ids from model.yml: {}", e);
                    }
                }
            }
            // Append any detected lookup tables from loaded data (ensuring stable order)
            let mut detected: Vec<String> = lookup_maps.keys().cloned().collect();
            detected.sort();
            for id in detected {
                dyn_ids.push(id);
            }
            // Exclude base/non-mapped tables
            let exclude = |id: &str| id == "period" || id == ".period" || id == "footnote" || id == "contacts";
            // Dedupe while preserving first occurrence order
            let mut seen: HashSet<String> = HashSet::new();
            dyn_ids.retain(|id| {
                if exclude(id) { return false; }
                if seen.contains(id) { return false; }
                seen.insert(id.clone());
                true
            });

            // For each dynamic lookup, we'll add <id>_code and <id>_name columns
            let mut dyn_code_builders: Vec<(String, StringBuilder)> = Vec::new();
            let mut dyn_name_builders: Vec<(String, StringBuilder)> = Vec::new();
            for id in &dyn_ids {
                dyn_code_builders.push((format!("{}_code", id), StringBuilder::new()));
                dyn_name_builders.push((format!("{}_name", id), StringBuilder::new()));
            }

            // Helper to derive a code for a given lookup id from series/observation
            let mut code_for = |id: &str, s: Option<&crate::data::model::Series>, obs: &crate::data::model::Observation| -> String {
                match id {
                    "area" => s.map(|sr| sr.area_code.clone()).unwrap_or_default(),
                    "item" => s.map(|sr| sr.item_code.clone()).unwrap_or_default(),
                    "seasonal" => s.map(|sr| sr.seasonal.clone()).unwrap_or_default(),
                    "periodicity" => s.map(|sr| sr.periodicity_code.clone()).unwrap_or_default(),
                    "base" => s.map(|sr| sr.base_code.clone()).unwrap_or_default(),
                    "period" | ".period" => obs.period().to_string(),
                    _ => String::new(),
                }
            };

            for obs in &context.observation_data {
                let sid = obs.series_id();
                let s_opt = series_map.get(sid).copied();

                let (title, scode) = if let Some(s) = s_opt {
                    (s.title().to_string(), s.survey_code().to_string())
                } else {
                    let sc = if sid.len() >= 2 { sid[0..2].to_uppercase() } else { String::new() };
                    (String::new(), sc)
                };

                // Period columns (base)
                let per_code = code_for("period", s_opt, obs);
                let per_name = lookup_maps
                    .get("period")
                    .and_then(|m| m.get(&per_code))
                    .cloned()
                    .or_else(|| lookup_maps.get(".period").and_then(|m| m.get(&per_code)).cloned())
                    .unwrap_or_default();

                b_series_id.append_value(sid);
                b_series_title.append_value(&title);
                b_survey_code.append_value(&scode);
                b_year.append_value(obs.year() as i32);
                b_period_code.append_value(&per_code);
                b_period_name.append_value(&per_name);
                if let Some(v) = obs.numeric_value() { b_value.append_value(v); } else { b_value.append_null(); }

                // Dynamic lookup columns
                for (i, id) in dyn_ids.iter().enumerate() {
                    let code = code_for(id, s_opt, obs);
                    let name = lookup_maps
                        .get(id)
                        .and_then(|m| m.get(&code))
                        .cloned()
                        .unwrap_or_default();

                    dyn_code_builders[i].1.append_value(&code);
                    dyn_name_builders[i].1.append_value(&name);
                }
            }

            // Finish base arrays
            let a_series_id = Arc::new(b_series_id.finish()) as ArrayRef;
            let a_series_title = Arc::new(b_series_title.finish()) as ArrayRef;
            let a_survey_code = Arc::new(b_survey_code.finish()) as ArrayRef;
            let a_year = Arc::new(b_year.finish()) as ArrayRef;
            let a_period_code = Arc::new(b_period_code.finish()) as ArrayRef;
            let a_period_name = Arc::new(b_period_name.finish()) as ArrayRef;
            let a_value = Arc::new(b_value.finish()) as ArrayRef;

            // Finish dynamic arrays and collect schema fields
            let mut schema_fields: Vec<Field> = vec![
                Field::new("series_id", DataType::Utf8, false),
                Field::new("series_title", DataType::Utf8, false),
                Field::new("survey_code", DataType::Utf8, false),
                Field::new("year", DataType::Int32, false),
                Field::new("period_code", DataType::Utf8, false),
                Field::new("period_name", DataType::Utf8, false),
                Field::new("value", DataType::Float64, true),
            ];

            let mut arrays: Vec<ArrayRef> = vec![
                a_series_id,
                a_series_title,
                a_survey_code,
                a_year,
                a_period_code,
                a_period_name,
                a_value,
            ];

            for (name, mut builder) in dyn_code_builders.into_iter() {
                schema_fields.push(Field::new(&name, DataType::Utf8, false));
                arrays.push(Arc::new(builder.finish()) as ArrayRef);
            }
            for (name, mut builder) in dyn_name_builders.into_iter() {
                schema_fields.push(Field::new(&name, DataType::Utf8, false));
                arrays.push(Arc::new(builder.finish()) as ArrayRef);
            }

            let schema = Arc::new(Schema::new(schema_fields));

            let batch = RecordBatch::try_new(
                schema.clone(),
                arrays,
            ).map_err(|e| ProcessingError::SystemError(format!("Failed to build record batch: {}", e)))?;

            let combined_path = Path::new(&final_dir).join("combined.parquet");
            let file = File::create(&combined_path)
                .map_err(|e| ProcessingError::SystemError(format!("Failed to create combined parquet file {}: {}", combined_path.display(), e)))?;

            let props = WriterProperties::builder().build();
            let mut aw = ArrowWriter::try_new(file, schema.clone(), Some(props))
                .map_err(|e| ProcessingError::SystemError(format!("Failed to create ArrowWriter: {}", e)))?;
            aw.write(&batch)
                .map_err(|e| ProcessingError::SystemError(format!("Failed to write batch to Parquet: {}", e)))?;
            aw.close()
                .map_err(|e| ProcessingError::SystemError(format!("Failed to close Parquet writer: {}", e)))?;
        }

        log::info!(
            "Writer stage completed: {} files, {} records",
            self.stats.files_written, self.stats.records_written
        );
        Ok(())
    }

    fn dependencies(&self) -> Vec<String> {
        // Writer can work independently - validator is optional
        vec![]
    }

    fn validate(&self, context: &ProcessingContext) -> Result<()> {
        // Writer validation - data will be populated by loader stage during execution
        // Just validate that we have input paths to work with
        if context.input_paths.is_empty() {
            return Err(ProcessingError::InvalidConfiguration(
                "No input paths provided for writing stage".to_string(),
            )
            .into());
        }
        Ok(())
    }

    async fn cleanup(&mut self, _context: &mut ProcessingContext) -> Result<()> {
        Ok(())
    }
}

fn infer_survey_code(context: &ProcessingContext) -> String {
    // Try from series data first
    if let Some(series) = context.series_data.first() {
        let code = series.survey_code().to_string();
        if !code.is_empty() {
            return code.to_string();
        }
    }

    // Try to infer from input paths (data/raw/bls/<code>/...)
    for p in &context.input_paths {
        let path = Path::new(p);
        if let (Some(parent), Some(grand)) = (path.parent(), path.parent().and_then(|p| p.parent())) {
            if let Some(grand_name) = grand.file_name() {
                if grand_name.to_string_lossy().eq_ignore_ascii_case("bls") {
                    if let Some(dir) = parent.file_name() {
                        return dir.to_string_lossy().to_uppercase();
                    }
                }
            }
        }
        if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
            if fname.len() >= 2 {
                return fname[0..2].to_uppercase();
            }
        }
    }

    "UNKNOWN".to_string()
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
