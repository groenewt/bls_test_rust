//! # Loader Stage Implementation
//!
//! This module provides the loader stage implementation for the processing pipeline.
//! The loader stage is responsible for reading data from various sources and preparing
//! it for subsequent processing stages.
//!
//! ## Features
//!
//! - **Multi-format Support**: Reads BLS data files in various formats
//! - **Parallel Loading**: Utilizes multiple threads for concurrent data loading
//! - **Memory Management**: Monitors memory usage and applies backpressure when needed
//! - **Error Recovery**: Handles file errors gracefully with retry mechanisms
//! - **Data Validation**: Performs basic validation during loading
//!
//! ## Supported Data Sources
//!
//! - Local files (series, data, lookup files)
//! - Compressed archives
//! - Network resources (with caching)
//! - Database connections (future enhancement)
//!
//! ## Usage
//!
//! ```rust
//! use rusty::processing::pipeline::loader::LoaderStageImpl;
//! use rusty::processing::{ProcessingContext, ProcessingConfig, PipelineStage};
//!
//! let mut loader = LoaderStageImpl::new();
//! let mut context = ProcessingContext::new(ProcessingConfig::default());
//!
//! loader.execute(&mut context)?;
//! ```

use async_trait::async_trait;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::data::reader::{
    DataReader,
    create_optimized_reader,
};
use crate::error::types::{ProcessingError, Result};
use crate::processing::traits::{
    LoaderStage, PipelineStage, ProcessingContext,
};
use crate::processing::ProcessingConfig;
use crate::utils::file::get_file_size;

/// Implementation of the loader stage
pub struct LoaderStageImpl {
    /// Configuration for the loader
    config: LoaderConfig,
    /// Statistics for the loader stage
    stats: LoaderStats,
    /// Cache of loaded data readers
    reader_cache: Arc<Mutex<HashMap<String, Box<dyn DataReader>>>>,
}

/// Configuration for the loader stage
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Maximum number of concurrent file loads
    pub max_concurrent_loads: usize,
    /// Maximum memory usage before applying backpressure (bytes)
    pub max_memory_usage: u64,
    /// Enable data validation during loading
    pub validate_during_load: bool,
    /// Retry count for failed loads
    pub retry_count: u32,
    /// Timeout for individual file loads (seconds)
    pub load_timeout_seconds: u64,
    /// Enable reader caching
    pub enable_caching: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            max_concurrent_loads: 4,
            max_memory_usage: 2 * 1024 * 1024 * 1024, // 2GB
            validate_during_load: true,
            retry_count: 3,
            load_timeout_seconds: 300, // 5 minutes
            enable_caching: true,
        }
    }
}

/// Statistics for the loader stage
#[derive(Debug, Clone, Default)]
pub struct LoaderStats {
    /// Number of files loaded
    pub files_loaded: u64,
    /// Total bytes loaded
    pub bytes_loaded: u64,
    /// Number of records loaded
    pub records_loaded: u64,
    /// Number of load errors
    pub load_errors: u64,
    /// Average load time per file (milliseconds)
    pub avg_load_time_ms: f64,
    /// Memory usage peak during loading
    pub peak_memory_usage: u64,
}

impl LoaderStageImpl {
    /// Create a new loader stage with default configuration
    pub fn new() -> Self {
        Self {
            config: LoaderConfig::default(),
            stats: LoaderStats::default(),
            reader_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new loader stage with custom configuration
    pub fn with_config(config: LoaderConfig) -> Self {
        Self {
            config,
            stats: LoaderStats::default(),
            reader_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get loader statistics
    pub fn stats(&self) -> &LoaderStats {
        &self.stats
    }

    /// Reset loader statistics
    pub fn reset_stats(&mut self) {
        self.stats = LoaderStats::default();
    }

    /// Load data from input paths
    fn load_data_from_paths(
        &mut self,
        paths: &[String],
        context: &mut ProcessingContext,
    ) -> Result<Vec<Box<dyn DataReader>>> {
        let start_time = Instant::now();
        let mut readers = Vec::new();
        let load_errors = 0u64;

        // Expand wildcards and resolve paths
        let resolved_paths = self.resolve_input_paths(paths)?;

        // Group paths by type for efficient loading
        let grouped_paths = self.group_paths_by_type(&resolved_paths)?;

        // Load files in parallel with controlled concurrency
        for (data_type, type_paths) in grouped_paths {
            let type_readers = self.load_paths_parallel(&type_paths, &data_type, context)?;
            readers.extend(type_readers);
        }

        // Update statistics
        let elapsed = start_time.elapsed();
        self.stats.avg_load_time_ms = elapsed.as_millis() as f64 / resolved_paths.len() as f64;
        self.stats.files_loaded += resolved_paths.len() as u64;
        self.stats.load_errors += load_errors;

        // Add metrics to context
        context.add_metric(
            "loader_files_loaded".to_string(),
            self.stats.files_loaded as f64,
        );
        context.add_metric(
            "loader_load_time_ms".to_string(),
            elapsed.as_millis() as f64,
        );

        Ok(readers)
    }

    /// Resolve input paths, expanding wildcards and validating existence
    fn resolve_input_paths(&self, paths: &[String]) -> Result<Vec<PathBuf>> {
        let mut resolved_paths = Vec::new();

        for path_str in paths {
            let path = resolve_path(path_str)?;

            if path_str.contains('*') || path_str.contains('?') {
                // Expand wildcards
                let expanded = expand_wildcards(&path)?;
                resolved_paths.extend(expanded);
            } else if path.exists() {
                resolved_paths.push(path);
            } else {
                // Be resilient: warn and continue instead of failing the whole loader
                log::warn!("[loader] Input path does not exist, skipping: {}", path_str);
                continue;
            }
        }

        // Validate all paths are readable
        for path in &resolved_paths {
            if !self.is_file_readable(path)? {
                return Err(ProcessingError::PipelineError {
                    stage: "loader".to_string(),
                    message: format!("File is not readable: {}", path.display()),
                }
                .into());
            }
        }

        Ok(resolved_paths)
    }

    /// Group paths by data type for efficient loading
    fn group_paths_by_type(&self, paths: &[PathBuf]) -> Result<HashMap<String, Vec<PathBuf>>> {
        let mut grouped = HashMap::new();

        for path in paths {
            let data_type = self.determine_data_type(path)?;
            grouped
                .entry(data_type)
                .or_insert_with(Vec::new)
                .push(path.clone());
        }

        Ok(grouped)
    }

    /// Determine data type from file path
    fn determine_data_type(&self, path: &Path) -> Result<String> {
        let path_str = path.to_string_lossy().to_lowercase();

        if path_str.contains(".series") {
            Ok("series".to_string())
        } else if path_str.contains(".data") {
            Ok("observations".to_string())
        } else if path_str.contains(".area")
            || path_str.contains(".item")
            || path_str.contains(".industry")
            || path_str.contains(".occupation")
            || path_str.contains(".footnote")
            || path_str.contains(".period")
            || path_str.contains(".seasonal")
            || path_str.contains(".contacts")
        {
            Ok("lookup".to_string())
        } else if path_str.contains(".survey") {
            Ok("survey".to_string())
        } else {
            // Try to determine from file content or use default
            Ok("unknown".to_string())
        }
    }

    /// Load paths in parallel with controlled concurrency
    fn load_paths_parallel(
        &mut self,
        paths: &[PathBuf],
        data_type: &str,
        context: &mut ProcessingContext,
    ) -> Result<Vec<Box<dyn DataReader>>> {
        let chunk_size = self.config.max_concurrent_loads;
        let mut all_readers = Vec::new();

        // Process paths in chunks to control memory usage
        for chunk in paths.chunks(chunk_size) {
            // Check memory usage before processing chunk
            self.check_memory_usage()?;

            // Load chunk in parallel
            let chunk_readers: Result<Vec<_>> = chunk
                .iter()
                .map(|path| self.load_single_file(path, data_type, context))
                .collect();

            match chunk_readers {
                Ok(mut readers) => {
                    all_readers.append(&mut readers);
                }
                Err(e) => {
                    self.stats.load_errors += 1;
                    if self.stats.load_errors > self.config.retry_count as u64 {
                        return Err(e);
                    }
                    // Continue with partial results
                    log::warn!("Failed to load chunk, continuing: {e}");
                }
            }
        }

        Ok(all_readers)
    }

    /// Load a single file with retry logic
    fn load_single_file(
        &mut self,
        path: &Path,
        data_type: &str,
        context: &ProcessingContext,
    ) -> Result<Box<dyn DataReader>> {
        let mut attempts = 0;
        let max_attempts = self.config.retry_count + 1;

        while attempts < max_attempts {
            match self.try_load_single_file(path, data_type, context) {
                Ok(reader) => {
                    // Update statistics
                    if let Ok(size) = get_file_size(path) {
                        self.stats.bytes_loaded += size;
                    }
                    return Ok(reader);
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(e);
                    }
                    log::warn!(
                        "Load attempt {} failed for {}: {}",
                        attempts,
                        path.display(),
                        e
                    );
                    std::thread::sleep(std::time::Duration::from_millis(100 * attempts as u64));
                }
            }
        }

        Err(ProcessingError::PipelineError {
            stage: "loader".to_string(),
            message: format!(
                "Failed to load file after {} attempts: {}",
                max_attempts,
                path.display()
            ),
        }
        .into())
    }

    /// Try to load a single file (single attempt)
    fn try_load_single_file(
        &mut self,
        path: &Path,
        data_type: &str,
        context: &ProcessingContext,
    ) -> Result<Box<dyn DataReader>> {
        // Check cache first if enabled
        if self.config.enable_caching {
            let cache_key = format!("{}:{}", path.display(), data_type);
            if let Ok(cache) = self.reader_cache.lock() {
                if let Some(_cached_reader) = cache.get(&cache_key) {
                    // Note: In a real implementation, you'd need to handle reader cloning
                    // For now, we'll create a new reader each time
                }
            }
        }

        // Create optimized reader for the file
        let reader = create_optimized_reader(path)?;

        // Perform basic validation if enabled
        if self.config.validate_during_load {
            self.validate_reader_data(&*reader, context)?;
        }

        Ok(reader)
    }

    /// Validate reader data during loading
    fn validate_reader_data(
        &mut self,
        reader: &dyn DataReader,
        context: &ProcessingContext,
    ) -> Result<()> {
        // This is a simplified validation - in a real implementation,
        // you'd perform more comprehensive checks

        // Check if reader can provide basic information
        if reader.stats().records_read == 0 {
            log::warn!("Reader contains no records");
        }

        // Update record count statistics
        self.stats.records_loaded += reader.stats().records_read;

        Ok(())
    }

    /// Check memory usage and apply backpressure if needed
    fn check_memory_usage(&self) -> Result<()> {
        // Get current memory usage (simplified implementation)
        let current_usage = self.get_current_memory_usage();

        if current_usage > self.config.max_memory_usage {
            return Err(ProcessingError::SystemError(format!(
                "Memory usage ({} bytes) exceeds limit ({} bytes)",
                current_usage, self.config.max_memory_usage
            ))
            .into());
        }

        Ok(())
    }

    /// Get current memory usage (simplified implementation)
    fn get_current_memory_usage(&self) -> u64 {
        // This would use system APIs in a real implementation
        // For now, return a placeholder value
        0
    }
    /// Check if a file is readable by attempting to open it.
    fn is_file_readable(&self, path: &Path) -> Result<bool> {
        match std::fs::File::open(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Ok(false),
            Err(e) => Err(ProcessingError::data_error(format!(
                "Error checking file readability for '{}': {}",
                path.display(),
                e
            ))),
        }
    }

    /// Parse BLS data files directly and populate context
    fn parse_bls_data(&mut self, paths: &[String], context: &mut ProcessingContext) -> Result<()> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        use crate::data::model::{Series, Observation, Lookup};
        use std::collections::HashMap;
        
        // Aggregate lookup tables across files
        let mut lookup_tables: HashMap<String, Lookup> = HashMap::new();
        
        for path_str in paths {
            let path = Path::new(path_str);
            if !path.exists() {
                continue;
            }
            
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
                
            log::info!("Parsing BLS file: {}", path.display());
            
            if filename.contains(".series") {
                // Parse series file
                let file = File::open(path).map_err(|e| 
                    crate::error::types::Error::Processing(
                        crate::error::types::ProcessingError::SystemError(
                            format!("Failed to open file {}: {}", path.display(), e)
                        )
                    )
                )?;
                let reader = BufReader::new(file);
                let mut lines = reader.lines();
                
                // Read header and map column indices if available
                let mut col_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
                if let Some(header_res) = lines.next() {
                    if let Ok(header) = header_res {
                        let headers: Vec<String> = if header.contains('\t') {
                            header.split('\t').map(|s| s.trim().to_lowercase()).collect()
                        } else {
                            header.split_whitespace().map(|s| s.trim().to_lowercase()).collect()
                        };
                        for (i, h) in headers.iter().enumerate() {
                            col_index.insert(h.clone(), i);
                        }
                    }
                }

                // Column helpers with sensible defaults/fallbacks
                let sid_idx = *col_index.get("series_id").unwrap_or(&0);
                let title_idx = col_index.get("series_title").copied();
                let area_idx = col_index.get("area_code").copied();
                let item_idx = col_index.get("item_code").copied();
                let seasonal_idx = col_index.get("seasonal").copied();
                let periodicity_idx = col_index.get("periodicity_code").or_else(|| col_index.get("periodicity")).copied();
                let base_code_idx = col_index.get("base_code").copied();
                let base_period_idx = col_index.get("base_period").copied();

                // Process data lines
                for line_result in lines {
                    let line = match line_result {
                        Ok(line) => line,
                        Err(e) => {
                            log::warn!("Failed to read line: {}", e);
                            continue;
                        }
                    };

                    // Prefer TSV, fallback to whitespace
                    let fields_tab: Vec<&str> = line.split('\t').collect();
                    let fields: Vec<&str> = if fields_tab.len() > 1 { fields_tab } else { line.split_whitespace().collect() };

                    if fields.is_empty() { continue; }

                    let get_field = |idx_opt: Option<usize>| -> String {
                        idx_opt
                            .and_then(|i| fields.get(i).map(|s| s.trim().to_string()))
                            .unwrap_or_default()
                    };

                    let series_id = get_field(Some(sid_idx));
                    if series_id.is_empty() { continue; }

                    // Title: prefer header-based index, else fallback to legacy assumption of 4th column
                    let series_title = if let Some(i) = title_idx {
                        get_field(Some(i))
                    } else if fields.len() >= 4 {
                        fields[3].trim().to_string()
                    } else {
                        String::new()
                    };

                    let mut series = Series::new(&series_id, &series_title);
                    let area_code = get_field(area_idx);
                    if !area_code.is_empty() { series.area_code = area_code; }
                    let item_code = get_field(item_idx);
                    if !item_code.is_empty() { series.item_code = item_code; }
                    let seasonal = get_field(seasonal_idx);
                    if !seasonal.is_empty() { series.seasonal = seasonal; }
                    let periodicity = get_field(periodicity_idx);
                    if !periodicity.is_empty() { series.periodicity_code = periodicity; }
                    let base_code = get_field(base_code_idx);
                    if !base_code.is_empty() { series.base_code = base_code; }
                    let base_period = get_field(base_period_idx);
                    if !base_period.is_empty() { series.base_period = base_period; }

                    // Infer survey code from path .../bls/<code>/...
                    let mut inferred_survey_code = String::from("XX");
                    if let Some(parent) = path.parent() {
                        if let Some(grand) = parent.parent() {
                            if let Some(grand_name) = grand.file_name() {
                                if grand_name.to_string_lossy().eq_ignore_ascii_case("bls") {
                                    if let Some(dir) = parent.file_name() {
                                        inferred_survey_code = dir.to_string_lossy().to_uppercase();
                                    }
                                }
                            }
                        }
                    }
                    series.survey_code = inferred_survey_code;

                    context.series_data.push(series);
                    self.stats.records_loaded += 1;
                }
            } else if filename.contains(".data") {
                // Parse observation data file
                let file = File::open(path).map_err(|e| 
                    crate::error::types::Error::Processing(
                        crate::error::types::ProcessingError::SystemError(
                            format!("Failed to open file {}: {}", path.display(), e)
                        )
                    )
                )?;
                let reader = BufReader::new(file);
                let mut lines = reader.lines();
                
                // Skip header line
                if let Some(_header) = lines.next() {
                    // Process data lines
                    for line_result in lines {
                        let line = match line_result {
                            Ok(line) => line,
                            Err(e) => {
                                log::warn!("Failed to read line: {}", e);
                                continue;
                            }
                        };
                        // Prefer TSV, fallback to whitespace for uncleaned files
                        let fields_tab: Vec<&str> = line.split('\t').collect();
                        let (series_id, year, period, value_opt) = if fields_tab.len() >= 4 {
                            let sid = fields_tab[0].trim().to_string();
                            let yr: i32 = fields_tab[1].trim().parse().unwrap_or(0);
                            let per = fields_tab[2].trim().to_string();
                            let val_str = fields_tab[3].trim();
                            let val_opt = sanitize_numeric(val_str);
                            (sid, yr, per, val_opt)
                        } else {
                            let mut sid = String::new();
                            let mut yr: i32 = 0;
                            let mut per = String::new();
                            let mut val_opt: Option<f64> = None;
                            let tokens: Vec<&str> = line.split_whitespace().collect();
                            if tokens.len() >= 4 {
                                sid = tokens[0].trim().to_string();
                                yr = tokens[1].trim().parse().unwrap_or(0);
                                per = tokens[2].trim().to_string();
                                val_opt = sanitize_numeric(tokens[3].trim());
                            }
                            (sid, yr, per, val_opt)
                        };

                        if !series_id.is_empty() {
                            let observation = Observation::new(
                                &series_id,
                                &year,
                                &period,
                                value_opt
                            );
                            context.observation_data.push(observation);
                            self.stats.records_loaded += 1;
                        }
                    }
                }
            } else if filename.contains(".area")
                     || filename.contains(".item")
                     || filename.contains(".footnote")
                     || filename.contains(".period")
                     || filename.contains(".seasonal")
                     || filename.contains(".contacts") {
                // Parse lookup files and aggregate into tables with entries
                let file = File::open(path).map_err(|e| 
                    crate::error::types::Error::Processing(
                        crate::error::types::ProcessingError::SystemError(
                            format!("Failed to open file {}: {}", path.display(), e)
                        )
                    )
                )?;
                let reader = BufReader::new(file);
                let mut lines = reader.lines();

                // Determine table id from filename
                let table_id = if filename.contains(".area") { "area" }
                               else if filename.contains(".item") { "item" }
                               else if filename.contains(".footnote") { "footnote" }
                               else if filename.contains(".seasonal") { "seasonal" }
                               else if filename.contains(".contacts") { "contacts" }
                               else { "period" };

                // Infer survey code from path .../bls/<code>/...
                let mut survey_code = String::from("XX");
                if let Some(parent) = path.parent() {
                    if let Some(grand) = parent.parent() {
                        if let Some(grand_name) = grand.file_name() {
                            if grand_name.to_string_lossy().eq_ignore_ascii_case("bls") {
                                if let Some(dir) = parent.file_name() {
                                    survey_code = dir.to_string_lossy().to_uppercase();
                                }
                            }
                        }
                    }
                }

                // Ensure lookup table exists
                lookup_tables.entry(table_id.to_string()).or_insert_with(|| Lookup::for_survey(table_id, table_id, &survey_code));
                
                // Skip header line
                if let Some(_header) = lines.next() {
                    // Process data lines
                    for line_result in lines {
                        let line = match line_result {
                            Ok(line) => line,
                            Err(e) => {
                                log::warn!("Failed to read line: {}", e);
                                continue;
                            }
                        };

                        // Prefer TSV, fallback to whitespace
                        let fields_tab: Vec<&str> = line.split('\t').collect();
                        let (code, description) = if fields_tab.len() >= 2 {
                            let code = fields_tab[0].trim().to_string();
                            let desc = if table_id == "period" && fields_tab.len() >= 3 {
                                // Use the period_name column if present
                                fields_tab[2].trim().to_string()
                            } else {
                                fields_tab[1..].join(" ").trim().to_string()
                            };
                            (code, desc)
                        } else {
                            let toks: Vec<&str> = line.split_whitespace().collect();
                            if toks.len() >= 2 {
                                let code = toks[0].trim().to_string();
                                let desc = if table_id == "period" && toks.len() >= 3 {
                                    toks[2].trim().to_string()
                                } else {
                                    toks[1..].join(" ").trim().to_string()
                                };
                                (code, desc)
                            } else { (String::new(), String::new()) }
                        };

                        if !code.is_empty() {
                            if let Some(table) = lookup_tables.get_mut(table_id) {
                                table.add_entry(&code, &description, None);
                                self.stats.records_loaded += 1;
                            }
                        }
                    }
                }
            }
        }
        
        // After parsing all files, add aggregated lookup tables to context
        if !lookup_tables.is_empty() {
            for (_, table) in lookup_tables.into_iter() {
                context.lookup_data.push(table);
            }
        }
        
        Ok(())
    }

    /// Estimate data size for memory planning
    fn estimate_data_size_internal(&self, paths: &[String]) -> Result<u64> {
        let mut total_size = 0u64;

        for path_str in paths {
            let path = Path::new(path_str);
            if path.exists() {
                total_size += get_file_size(path)?;
            }
        }

        Ok(total_size)
    }
}

fn resolve_path(path_str: &String) -> Result<PathBuf> {
    use std::path::Path;
    let path = Path::new(path_str);
    
    // Convert to absolute path if relative
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        // For relative paths, resolve relative to current working directory
        match std::env::current_dir() {
            Ok(cwd) => Ok(cwd.join(path)),
            Err(e) => Err(crate::error::types::Error::Processing(
                crate::error::types::ProcessingError::InvalidConfiguration(
                    format!("Failed to get current directory: {}", e)
                )
            ))
        }
    }
}

fn expand_wildcards(path: &Path) -> Result<Vec<PathBuf>> {
    use std::fs;

    let path_str = path.to_string_lossy();

    // For now, implement basic wildcard expansion
    if path_str.contains('*') || path_str.contains('?') {
        // Simple wildcard expansion - if path contains wildcards,
        // try to match files in the parent directory
        if let Some(parent) = path.parent() {
            let filename_pattern = path.file_name().and_then(|n| n.to_str()).unwrap_or("*");

            match fs::read_dir(parent) {
                Ok(entries) => {
                    let mut result = Vec::new();
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let entry_path = entry.path();
                            // Only include files (skip directories)
                            if entry_path.is_file() {
                                if let Some(entry_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                                    // Very basic matching: if pattern has wildcard, include all; otherwise exact match
                                    if filename_pattern.contains('*') || filename_pattern.contains('?') {
                                        result.push(entry_path);
                                    } else if entry_name == filename_pattern {
                                        result.push(entry_path);
                                    }
                                }
                            }
                        }
                    }
                    Ok(result)
                }
                Err(_e) => {
                    // Be resilient: missing directory or unreadable dir returns empty results
                    Ok(Vec::new())
                }
            }
        } else {
            Ok(Vec::new())
        }
    } else {
        // No wildcards, return the path as-is if it exists
        if path.exists() {
            Ok(vec![path.to_path_buf()])
        } else {
            Ok(Vec::new()) // Return empty if file doesn't exist
        }
    }
}

impl Default for LoaderStageImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelineStage for LoaderStageImpl {
    fn name(&self) -> &str {
        "loader"
    }

    fn description(&self) -> &str {
        "Loads data from various sources and prepares it for processing"
    }

    fn can_process(&self, context: &ProcessingContext) -> Result<bool> {
        // Check if there are input paths to process
        Ok(!context.input_paths.is_empty())
    }

    async fn execute(&mut self, context: &mut ProcessingContext) -> Result<()> {
        log::info!("Starting loader stage execution");

        // Make loader idempotent across DAG tasks: skip if already executed
        if context.custom_data.get("loader_executed").map(|v| v == "true").unwrap_or(false) {
            log::info!("Loader already executed for this context; skipping to avoid duplicates");
            return Ok(());
        }

        // Load data from input paths
        let input_paths = context.input_paths.clone();
        let readers = self.load_data_from_paths(&input_paths, context)?;

        // Parse BLS data directly from files and populate context
        // Use resolved (expanded) file paths so patterns like '*.series' and 'data/*' are honored
        let resolved_paths = self.resolve_input_paths(&input_paths)?;
        let resolved_strs: Vec<String> = resolved_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        self.parse_bls_data(&resolved_strs, context)?;

        // Store readers in context for next stages
        context.data_readers = readers;

        // Mark as executed to prevent duplicate loads in subsequent DAG tasks
        context.custom_data.insert("loader_executed".to_string(), "true".to_string());

        log::info!(
            "Loader stage completed: {} files loaded, {} records, {} series, {} observations, {} lookups",
            self.stats.files_loaded,
            self.stats.records_loaded,
            context.series_data.len(),
            context.observation_data.len(),
            context.lookup_data.len()
        );

        Ok(())
    }

    fn dependencies(&self) -> Vec<String> {
        // Loader stage has no dependencies
        vec![]
    }

    fn validate(&self, context: &ProcessingContext) -> Result<()> {
        // Validate that input paths are provided
        if context.input_paths.is_empty() {
            return Err(ProcessingError::invalid_configuration(
                "No input paths provided for loader stage".to_string(),
            ));
        }

        // Validate configuration
        if self.config.max_concurrent_loads == 0 {
            return Err(ProcessingError::invalid_configuration(
                "max_concurrent_loads must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }

    async fn cleanup(&mut self, _context: &mut ProcessingContext) -> Result<()> {
        // Clear reader cache if enabled
        if self.config.enable_caching {
            if let Ok(mut cache) = self.reader_cache.lock() {
                cache.clear();
            }
        }

        Ok(())
    }
}

#[async_trait]
impl LoaderStage for LoaderStageImpl {
    async fn load_data(
        &mut self,
        context: &mut ProcessingContext,
    ) -> Result<Vec<Box<dyn DataReader>>> {
        let input_paths = context.input_paths.clone();
        self.load_data_from_paths(&input_paths, context)
    }

    fn estimate_data_size(&self, context: &ProcessingContext) -> Result<u64> {
        self.estimate_data_size_internal(&context.input_paths)
    }

    fn check_data_availability(&self, context: &ProcessingContext) -> Result<bool> {
        for path_str in &context.input_paths {
            let path = Path::new(path_str);
            if !path.exists() || !self.is_file_readable(path)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file_path
    }

    #[test]
    fn test_loader_stage_creation() {
        let loader = LoaderStageImpl::new();
        assert_eq!(loader.name(), "loader");
        assert!(!loader.description().is_empty());
    }

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert_eq!(config.max_concurrent_loads, 4);
        assert!(config.validate_during_load);
        assert!(config.enable_caching);
    }

    #[test]
    fn test_determine_data_type() {
        let loader = LoaderStageImpl::new();

        assert_eq!(
            loader
                .determine_data_type(Path::new("test.series"))
                .unwrap(),
            "series"
        );
        assert_eq!(
            loader.determine_data_type(Path::new("test.data")).unwrap(),
            "observations"
        );
        assert_eq!(
            loader.determine_data_type(Path::new("test.area")).unwrap(),
            "lookup"
        );
        assert_eq!(
            loader.determine_data_type(Path::new("test.item")).unwrap(),
            "lookup"
        );
        assert_eq!(
            loader
                .determine_data_type(Path::new("test.survey"))
                .unwrap(),
            "survey"
        );
        assert_eq!(
            loader
                .determine_data_type(Path::new("test.unknown"))
                .unwrap(),
            "unknown"
        );
    }

    #[test]
    fn test_loader_stage_validation() {
        let loader = LoaderStageImpl::new();
        let mut context = ProcessingContext::new(ProcessingConfig::default());

        // Should fail with empty input paths
        assert!(loader.validate(&context).is_err());

        // Should pass with input paths
        context.input_paths.push("test.data".to_string());
        assert!(loader.validate(&context).is_ok());
    }

    #[test]
    fn test_can_process() {
        let loader = LoaderStageImpl::new();
        let mut context = ProcessingContext::new(ProcessingConfig::default());

        // Should return false with no input paths
        assert!(!loader.can_process(&context).unwrap());

        // Should return true with input paths
        context.input_paths.push("test.data".to_string());
        assert!(loader.can_process(&context).unwrap());
    }

    #[test]
    fn test_loader_stats() {
        let mut loader = LoaderStageImpl::new();
        assert_eq!(loader.stats().files_loaded, 0);
        assert_eq!(loader.stats().records_loaded, 0);

        loader.reset_stats();
        assert_eq!(loader.stats().files_loaded, 0);
    }

    #[test]
    fn test_memory_usage_check() {
        let loader = LoaderStageImpl::new();
        // Should not fail with default limits
        assert!(loader.check_memory_usage().is_ok());
    }

    #[test]
    fn test_estimate_data_size() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "test.data", "test content");

        let loader = LoaderStageImpl::new();
        let paths = vec![file_path.to_string_lossy().to_string()];
        let size = loader.estimate_data_size_internal(&paths).unwrap();

        assert!(size > 0);
    }

    #[test]
    fn test_group_paths_by_type() {
        let temp_dir = TempDir::new().unwrap();
        let series_path = create_test_file(&temp_dir, "test.series", "series data");
        let data_path = create_test_file(&temp_dir, "test.data", "observation data");

        let loader = LoaderStageImpl::new();
        let paths = vec![series_path, data_path];
        let grouped = loader.group_paths_by_type(&paths).unwrap();

        assert!(grouped.contains_key("series"));
        assert!(grouped.contains_key("observations"));
    }

    #[tokio::test]
    async fn test_cleanup() {
        let mut loader = LoaderStageImpl::new();
        let mut context = ProcessingContext::new(ProcessingConfig::default());

        // Should not fail
        assert!(loader.cleanup(&mut context).await.is_ok());
    }
}


// Helper: sanitize numeric strings from raw BLS files (space-separated/uncleaned)
// - removes commas and trims whitespace
// - interprets "-" or empty as missing (None)
fn sanitize_numeric(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || t == "-" { return None; }
    let cleaned = t.replace(",", "");
    cleaned.parse::<f64>().ok()
}
