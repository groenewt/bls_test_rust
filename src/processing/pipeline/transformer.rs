//! # Transformer Stage Implementation
//!
//! This module provides the transformer stage implementation for the processing pipeline.
//! The transformer stage applies data transformations, business logic, and data enrichment
//! to the loaded data before validation and output.
//!
//! ## Features
//!
//! - **Flexible Transformations**: Supports field mapping, type conversion, and value transformation
//! - **Parallel Processing**: Utilizes multiple threads for concurrent data transformation
//! - **Rule-Based System**: Configurable transformation rules with conditional logic
//! - **Data Enrichment**: Adds computed fields and derived values
//! - **Performance Optimization**: Efficient batch processing with memory management
//!
//! ## Transformation Types
//!
//! - **Field Mapping**: Rename and restructure data fields
//! - **Type Conversion**: Convert data types (string to numeric, date parsing, etc.)
//! - **Value Transformation**: Apply functions to transform values
//! - **Aggregation**: Compute summary statistics and aggregated values
//! - **Filtering**: Remove or flag records based on criteria
//! - **Custom Logic**: Apply custom business rules and calculations
//!
//! ## Usage
//!
//! ```rust
//! use rusty::processing::pipeline::transformer::TransformerStageImpl;
//! use rusty::processing::{ProcessingContext, ProcessingConfig};
//!
//! let mut transformer = TransformerStageImpl::new();
//! let mut context = ProcessingContext::new(crate::processing::traits::ProcessingConfig::default());
//! 
//! transformer.execute(&mut context)?;
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use async_trait::async_trait;
use rayon::prelude::*;

use crate::processing::traits::{
    PipelineStage, TransformerStage, ProcessingContext,
    TransformationRule, TransformationRuleType,
};
use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::error::types::{ProcessingError, Result};
use crate::utils::format::{parse_date, format_currency, normalize_text};


/// Implementation of the transformer stage
pub struct TransformerStageImpl {
    /// Configuration for the transformer
    config: TransformerConfig,
    /// Statistics for the transformer stage
    stats: TransformerStats,
    /// Transformation rules cache
    rules_cache: Arc<Mutex<HashMap<String, Vec<TransformationRule>>>>,
    /// Custom transformation functions
    custom_functions: HashMap<String, Box<dyn Fn(&str) -> Result<String> + Send + Sync>>,
}

/// Configuration for the transformer stage
#[derive(Debug, Clone)]
pub struct TransformerConfig {
    /// Maximum number of concurrent transformations
    pub max_concurrent_transforms: usize,
    /// Enable parallel processing
    pub enable_parallel_processing: bool,
    /// Batch size for processing
    pub batch_size: usize,
    /// Enable transformation caching
    pub enable_caching: bool,
    /// Maximum cache size (number of entries)
    pub max_cache_size: usize,
    /// Enable data enrichment
    pub enable_enrichment: bool,
    /// Transformation timeout (seconds)
    pub transformation_timeout_seconds: u64,
}

impl Default for TransformerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_transforms: 4,
            enable_parallel_processing: true,
            batch_size: 1000,
            enable_caching: true,
            max_cache_size: 10000,
            enable_enrichment: true,
            transformation_timeout_seconds: 60,
        }
    }
}

/// Statistics for the transformer stage
#[derive(Debug, Clone, Default)]
pub struct TransformerStats {
    /// Number of records transformed
    pub records_transformed: u64,
    /// Number of transformation rules applied
    pub rules_applied: u64,
    /// Number of transformation errors
    pub transformation_errors: u64,
    /// Average transformation time per record (microseconds)
    pub avg_transform_time_us: f64,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
    /// Number of enriched records
    pub enriched_records: u64,
}

/// Transformation context for a single record
#[derive(Debug, Clone)]
pub struct TransformationContext {
    /// Original record data
    pub original_data: HashMap<String, String>,
    /// Transformed record data
    pub transformed_data: HashMap<String, String>,
    /// Metadata about the transformation
    pub metadata: HashMap<String, String>,
    /// Transformation errors
    pub errors: Vec<String>,
}

impl TransformationContext {
    /// Create new transformation context
    pub fn new(original_data: HashMap<String, String>) -> Self {
        Self {
            transformed_data: original_data.clone(),
            original_data,
            metadata: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Add transformation error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    /// Check if transformation was successful
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get transformed value
    pub fn get_transformed(&self, key: &str) -> Option<&String> {
        self.transformed_data.get(key)
    }

    /// Set transformed value
    pub fn set_transformed(&mut self, key: String, value: String) {
        self.transformed_data.insert(key, value);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

impl TransformerStageImpl {
    /// Create a new transformer stage with default configuration
    pub fn new() -> Self {
        Self {
            config: TransformerConfig::default(),
            stats: TransformerStats::default(),
            rules_cache: Arc::new(Mutex::new(HashMap::new())),
            custom_functions: HashMap::new(),
        }
    }

    /// Create a new transformer stage with custom configuration
    pub fn with_config(config: TransformerConfig) -> Self {
        Self {
            config,
            stats: TransformerStats::default(),
            rules_cache: Arc::new(Mutex::new(HashMap::new())),
            custom_functions: HashMap::new(),
        }
    }

    /// Get transformer statistics
    pub fn stats(&self) -> &TransformerStats {
        &self.stats
    }

    /// Reset transformer statistics
    pub fn reset_stats(&mut self) {
        self.stats = TransformerStats::default();
    }

    /// Add custom transformation function
    pub fn add_custom_function<F>(&mut self, name: String, func: F)
    where
        F: Fn(&str) -> Result<String> + Send + Sync + 'static,
    {
        self.custom_functions.insert(name, Box::new(func));
    }

    /// Transform series data
    fn transform_series_data(&mut self, series: Vec<Series>, context: &mut ProcessingContext) -> Result<Vec<Series>> {
        let start_time = Instant::now();
        let rules = self.get_transformation_rules_for_type("series")?;
        
        let transformed_series = if self.config.enable_parallel_processing {
            series
                .into_par_iter()
                .map(|s| self.transform_single_series(s, &rules))
                .collect::<Result<Vec<_>>>()?
        } else {
            series
                .into_iter()
                .map(|s| self.transform_single_series(s, &rules))
                .collect::<Result<Vec<_>>>()?
        };

        // Update statistics
        let elapsed = start_time.elapsed();
        self.stats.records_transformed += transformed_series.len() as u64;
        self.stats.avg_transform_time_us = elapsed.as_micros() as f64 / transformed_series.len() as f64;
        self.stats.rules_applied += (rules.len() * transformed_series.len()) as u64;

        // Add metrics to context
        context.add_metric("transformer_series_processed".to_string(), transformed_series.len() as f64);
        context.add_metric("transformer_series_time_ms".to_string(), elapsed.as_millis() as f64);

        Ok(transformed_series)
    }

    /// Transform a single series record
    fn transform_single_series(&self, mut series: Series, rules: &[TransformationRule]) -> Result<Series> {
        // Create transformation context
        let mut original_data = HashMap::new();
        original_data.insert("series_id".to_string(), series.series_id.clone());
        original_data.insert("title".to_string(), series.title.clone());
        original_data.insert("area_code".to_string(), series.base_code.clone());
        original_data.insert("item_code".to_string(), series.item_code.clone());

        let mut transform_context = TransformationContext::new(original_data);

        // Apply transformation rules
        for rule in rules {
            self.apply_transformation_rule(rule, &mut transform_context)?;
        }

        // Update series with transformed data
        if let Some(title) = transform_context.get_transformed("title") {
            series.title = title.clone();
        }
        if let Some(area_code) = transform_context.get_transformed("area_code") {
            series.base_code = area_code.clone();
        }
        if let Some(item_code) = transform_context.get_transformed("item_code") {
            series.item_code = item_code.clone();
        }

        // Apply enrichment if enabled
        if self.config.enable_enrichment {
            self.enrich_series(&mut series, &transform_context)?;
        }

        Ok(series)
    }

    /// Transform observations data
    fn transform_observations_data(&mut self, observations: Vec<Observation>, context: &mut ProcessingContext) -> Result<Vec<Observation>> {
        let start_time = Instant::now();
        let rules = self.get_transformation_rules_for_type("observations")?;
        
        let transformed_observations = if self.config.enable_parallel_processing {
            observations
                .into_par_iter()
                .map(|o| self.transform_single_observation(o, &rules))
                .collect::<Result<Vec<_>>>()?
        } else {
            observations
                .into_iter()
                .map(|o| self.transform_single_observation(o, &rules))
                .collect::<Result<Vec<_>>>()?
        };

        // Update statistics
        let elapsed = start_time.elapsed();
        self.stats.records_transformed += transformed_observations.len() as u64;
        self.stats.rules_applied += (rules.len() * transformed_observations.len()) as u64;

        // Add metrics to context
        context.add_metric("transformer_observations_processed".to_string(), transformed_observations.len() as f64);
        context.add_metric("transformer_observations_time_ms".to_string(), elapsed.as_millis() as f64);

        Ok(transformed_observations)
    }

    /// Transform a single observation record
    fn transform_single_observation(&self, mut observation: Observation, rules: &[TransformationRule]) -> Result<Observation> {
        // Create transformation context
        let mut original_data = HashMap::new();
        original_data.insert("series_id".to_string(), observation.series_id.clone());
        original_data.insert("year".to_string(), observation.year.to_string());
        original_data.insert("period".to_string(), observation.period.clone());
        original_data.insert("value".to_string(), observation.value.format_value(None));

        let mut transform_context = TransformationContext::new(original_data);

        // Apply transformation rules
        for rule in rules {
            self.apply_transformation_rule(rule, &mut transform_context)?;
        }

        // Update observation with transformed data
        if let Some(value) = transform_context.get_transformed("value") {
            if let Ok(numeric_value) = value.parse::<f64>() {
                observation.set_value(Some(numeric_value));
            }
        }
        if let Some(period) = transform_context.get_transformed("period") {
            observation.period = period.clone();
        }

        // Apply enrichment if enabled
        if self.config.enable_enrichment {
            self.enrich_observation(&mut observation, &transform_context)?;
        }

        Ok(observation)
    }

    /// Transform lookups data
    fn transform_lookups_data(&mut self, lookups: Vec<Lookup>, context: &mut ProcessingContext) -> Result<Vec<Lookup>> {
        let start_time = Instant::now();
        let rules = self.get_transformation_rules_for_type("lookups")?;
        
        let transformed_lookups = if self.config.enable_parallel_processing {
            lookups
                .into_par_iter()
                .map(|l| self.transform_single_lookup(l, &rules))
                .collect::<Result<Vec<_>>>()?
        } else {
            lookups
                .into_iter()
                .map(|l| self.transform_single_lookup(l, &rules))
                .collect::<Result<Vec<_>>>()?
        };

        // Update statistics
        let elapsed = start_time.elapsed();
        self.stats.records_transformed += transformed_lookups.len() as u64;
        self.stats.rules_applied += (rules.len() * transformed_lookups.len()) as u64;

        // Add metrics to context
        context.add_metric("transformer_lookups_processed".to_string(), transformed_lookups.len() as f64);
        context.add_metric("transformer_lookups_time_ms".to_string(), elapsed.as_millis() as f64);

        Ok(transformed_lookups)
    }

    /// Transform a single lookup record
    fn transform_single_lookup(&self, mut lookup: Lookup, rules: &[TransformationRule]) -> Result<Lookup> {
        // Create transformation context
        let mut original_data = HashMap::new();
        original_data.insert("code".to_string(), lookup.table_id.clone());
        original_data.insert("text".to_string(), lookup.table_name.clone());

        let mut transform_context = TransformationContext::new(original_data);

        // Apply transformation rules
        for rule in rules {
            self.apply_transformation_rule(rule, &mut transform_context)?;
        }

        // Update lookup with transformed data
        if let Some(text) = transform_context.get_transformed("text") {
            lookup.table_name = text.clone();
        }

        // Apply enrichment if enabled
        if self.config.enable_enrichment {
            self.enrich_lookup(&mut lookup, &transform_context)?;
        }

        Ok(lookup)
    }

    /// Apply a single transformation rule
    fn apply_transformation_rule(&self, rule: &TransformationRule, context: &mut TransformationContext) -> Result<()> {
        match &rule.rule_type {
            TransformationRuleType::FieldMapping => {
                self.apply_field_mapping(rule, context)
            }
            TransformationRuleType::TypeConversion => {
                self.apply_type_conversion(rule, context)
            }
            TransformationRuleType::ValueTransformation => {
                self.apply_value_transformation(rule, context)
            }
            TransformationRuleType::Aggregation => {
                self.apply_aggregation(rule, context)
            }
            TransformationRuleType::Filtering => {
                self.apply_filtering(rule, context)
            }
            TransformationRuleType::Custom(function_name) => {
                self.apply_custom_transformation(function_name, rule, context)
            }
        }
    }

    /// Apply field mapping transformation
    fn apply_field_mapping(&self, rule: &TransformationRule, context: &mut TransformationContext) -> Result<()> {
        if let (Some(source_field), Some(target_field)) = (rule.source_field.as_ref(), rule.target_field.as_ref()) {
            if let Some(value) = context.original_data.get(source_field) {
                context.set_transformed(target_field.clone(), value.clone());
            }
        }
        Ok(())
    }

    /// Apply type conversion transformation
    fn apply_type_conversion(&self, rule: &TransformationRule, context: &mut TransformationContext) -> Result<()> {
        if let Some(field) = rule.source_field.as_ref() {
            if let Some(value) = context.get_transformed(field) {
                let converted_value = match rule.parameters.get("target_type").map(|s| s.as_str()) {
                    Some("numeric") => self.convert_to_numeric(value)?,
                    Some("date") => self.convert_to_date(value)?,
                    Some("currency") => self.convert_to_currency(value)?,
                    Some("text") => self.convert_to_text(value)?,
                    _ => value.clone(),
                };
                context.set_transformed(field.clone(), converted_value);
            }
        }
        Ok(())
    }

    /// Apply value transformation
    fn apply_value_transformation(&self, rule: &TransformationRule, context: &mut TransformationContext) -> Result<()> {
        if let Some(field) = rule.source_field.as_ref() {
            if let Some(value) = context.get_transformed(field) {
                let transformed_value = match rule.parameters.get("operation").map(|s| s.as_str()) {
                    Some("uppercase") => value.to_uppercase(),
                    Some("lowercase") => value.to_lowercase(),
                    Some("trim") => value.trim().to_string(),
                    Some("normalize") => normalize_text(value)?,
                    _ => value.clone(),
                };
                context.set_transformed(field.clone(), transformed_value);
            }
        }
        Ok(())
    }

    /// Apply aggregation transformation
    fn apply_aggregation(&self, _rule: &TransformationRule, _context: &mut TransformationContext) -> Result<()> {
        // Aggregation transformations would be more complex and context-dependent
        // This is a placeholder for future implementation
        Ok(())
    }

    /// Apply filtering transformation
    fn apply_filtering(&self, rule: &TransformationRule, context: &mut TransformationContext) -> Result<()> {
        if let Some(field) = rule.source_field.as_ref() {
            if let Some(value) = context.get_transformed(field) {
                if let Some(filter_value) = rule.parameters.get("filter_value") {
                    if value == filter_value {
                        context.add_metadata("filtered".to_string(), "true".to_string());
                    }
                }
            }
        }
        Ok(())
    }

    /// Apply custom transformation
    fn apply_custom_transformation(&self, function_name: &str, rule: &TransformationRule, context: &mut TransformationContext) -> Result<()> {
        if let Some(func) = self.custom_functions.get(function_name) {
            if let Some(field) = rule.source_field.as_ref() {
                if let Some(value) = context.get_transformed(field) {
                    let transformed_value = func(value)?;
                    context.set_transformed(field.clone(), transformed_value);
                }
            }
        }
        Ok(())
    }

    /// Convert value to numeric format
    fn convert_to_numeric(&self, value: &str) -> Result<String> {
        let cleaned = value.replace(",", "").replace("$", "");
        match cleaned.parse::<f64>() {
            Ok(num) => Ok(num.to_string()),
            Err(_) => Ok("0".to_string()), // Default to 0 for invalid numbers
        }
    }

    /// Convert value to date format
    fn convert_to_date(&self, value: &str) -> Result<String> {
        parse_date(value).map(|d| d.to_string())
    }

    /// Convert value to currency format
    fn convert_to_currency(&self, value: &str) -> Result<String> {
        let num: f64 = value.parse().unwrap_or(0.0);
        Ok(format_currency(num))
    }

    /// Convert value to text format
    fn convert_to_text(&self, value: &str) -> Result<String> {
        Ok(value.to_string())
    }

    /// Get transformation rules for a specific data type
    fn get_transformation_rules_for_type(&self, data_type: &str) -> Result<Vec<TransformationRule>> {
        // Check cache first
        if let Ok(cache) = self.rules_cache.lock() {
            if let Some(rules) = cache.get(data_type) {
                return Ok(rules.clone());
            }
        }

        // Load default rules for the data type
        let rules = self.load_default_transformation_rules(data_type)?;

        // Cache the rules
        if let Ok(mut cache) = self.rules_cache.lock() {
            cache.insert(data_type.to_string(), rules.clone());
        }

        Ok(rules)
    }

    /// Load default transformation rules for a data type
    fn load_default_transformation_rules(&self, data_type: &str) -> Result<Vec<TransformationRule>> {
        let mut rules = Vec::new();

        match data_type {
            "series" => {
                // Default series transformation rules
                rules.push(TransformationRule {
                    name: "normalize_title".to_string(),
                    description: "Normalize series title".to_string(),
                    rule_type: TransformationRuleType::ValueTransformation,
                    source_field: Some("title".to_string()),
                    target_field: Some("title".to_string()),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("operation".to_string(), "normalize".to_string());
                        params
                    },
                    enabled: true,
                    priority: 1,
                });
            }
            "observations" => {
                // Default observation transformation rules
                rules.push(TransformationRule {
                    name: "convert_value_to_numeric".to_string(),
                    description: "Convert observation value to numeric".to_string(),
                    rule_type: TransformationRuleType::TypeConversion,
                    source_field: Some("value".to_string()),
                    target_field: Some("value".to_string()),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("target_type".to_string(), "numeric".to_string());
                        params
                    },
                    enabled: true,
                    priority: 1,
                });
            }
            "lookups" => {
                // Default lookup transformation rules
                rules.push(TransformationRule {
                    name: "normalize_lookup_text".to_string(),
                    description: "Normalize lookup text".to_string(),
                    rule_type: TransformationRuleType::ValueTransformation,
                    source_field: Some("text".to_string()),
                    target_field: Some("text".to_string()),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("operation".to_string(), "normalize".to_string());
                        params
                    },
                    enabled: true,
                    priority: 1,
                });
            }
            _ => {
                // No default rules for unknown types
            }
        }

        Ok(rules)
    }

    /// Enrich series data with additional computed fields
    fn enrich_series(&self, _series: &mut Series, _context: &TransformationContext) -> Result<()> {
        // Add computed fields or derived values
        // This is a placeholder for future enhancement
        Ok(())
    }

    /// Enrich observation data with additional computed fields
    fn enrich_observation(&self, _observation: &mut Observation, _context: &TransformationContext) -> Result<()> {
        // Add computed fields or derived values
        // This is a placeholder for future enhancement
        Ok(())
    }

    /// Enrich lookup data with additional computed fields
    fn enrich_lookup(&self, _lookup: &mut Lookup, _context: &TransformationContext) -> Result<()> {
        // Add computed fields or derived values
        // This is a placeholder for future enhancement
        Ok(())
    }
}

impl Default for TransformerStageImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PipelineStage for TransformerStageImpl {
    fn name(&self) -> &str {
        "transformer"
    }

    fn description(&self) -> &str {
        "Applies data transformations and business logic to loaded data"
    }

    fn can_process(&self, context: &ProcessingContext) -> Result<bool> {
        // Check if there are data readers to process
        Ok(!context.data_readers.is_empty())
    }

    async fn execute(&mut self, _context: &mut ProcessingContext) -> Result<()> {
        log::info!("Starting transformer stage execution");
        
        // Process different types of data
        // Note: This is a simplified implementation
        // In a real system, you'd extract data from readers and transform it
        
        log::info!("Transformer stage completed: {} records transformed", 
                  self.stats.records_transformed);
        
        Ok(())
    }

    fn dependencies(&self) -> Vec<String> {
        // Transformer depends on loader stage
        vec!["loader".to_string()]
    }

    fn validate(&self, context: &ProcessingContext) -> Result<()> {
        // Validate that data readers are available
        if context.data_readers.is_empty() {
            return Err(ProcessingError::InvalidConfiguration(
                "No data readers available for transformer stage".to_string()
            ).into());
        }

        // Validate configuration
        if self.config.batch_size == 0 {
            return Err(ProcessingError::InvalidConfiguration(
                "batch_size must be greater than 0".to_string()
            ).into());
        }

        Ok(())
    }

    async fn cleanup(&mut self, _context: &mut ProcessingContext) -> Result<()> {
        // Clear transformation rules cache
        if let Ok(mut cache) = self.rules_cache.lock() {
            cache.clear();
        }
        
        Ok(())
    }
}

#[async_trait]
impl TransformerStage for TransformerStageImpl {
    async fn transform_series(&mut self, series: Vec<Series>, context: &mut ProcessingContext) -> Result<Vec<Series>> {
        self.transform_series_data(series, context)
    }

    async fn transform_observations(&mut self, observations: Vec<Observation>, context: &mut ProcessingContext) -> Result<Vec<Observation>> {
        self.transform_observations_data(observations, context)
    }

    async fn transform_lookups(&mut self, lookups: Vec<Lookup>, context: &mut ProcessingContext) -> Result<Vec<Lookup>> {
        self.transform_lookups_data(lookups, context)
    }

    async fn transform_survey(&mut self, survey: Survey, _context: &mut ProcessingContext) -> Result<Survey> {
        // Survey transformation is typically simpler
        Ok(survey)
    }

    fn get_transformation_rules(&self) -> Vec<TransformationRule> {
        // Return all cached transformation rules
        let mut all_rules = Vec::new();
        
        if let Ok(cache) = self.rules_cache.lock() {
            for rules in cache.values() {
                all_rules.extend(rules.clone());
            }
        }
        
        all_rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformer_stage_creation() {
        let transformer = TransformerStageImpl::new();
        assert_eq!(transformer.name(), "transformer");
        assert!(!transformer.description().is_empty());
    }

    #[test]
    fn test_transformer_config_default() {
        let config = TransformerConfig::default();
        assert_eq!(config.max_concurrent_transforms, 4);
        assert!(config.enable_parallel_processing);
        assert_eq!(config.batch_size, 1000);
    }

    #[test]
    fn test_transformation_context() {
        let mut original_data = HashMap::new();
        original_data.insert("field1".to_string(), "value1".to_string());
        
        let mut context = TransformationContext::new(original_data);
        assert!(context.is_successful());
        
        context.add_error("Test error".to_string());
        assert!(!context.is_successful());
        
        context.set_transformed("field2".to_string(), "value2".to_string());
        assert_eq!(context.get_transformed("field2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_convert_to_numeric() {
        let transformer = TransformerStageImpl::new();
        
        assert_eq!(transformer.convert_to_numeric("123.45").unwrap(), "123.45");
        assert_eq!(transformer.convert_to_numeric("$1,234.56").unwrap(), "1234.56");
        assert_eq!(transformer.convert_to_numeric("invalid").unwrap(), "0");
    }

    #[test]
    fn test_convert_to_currency() {
        let transformer = TransformerStageImpl::new();
        
        let result = transformer.convert_to_currency("1234.56").unwrap();
        assert!(result.contains("1234.56")); // Basic check
    }

    #[test]
    fn test_load_default_transformation_rules() {
        let transformer = TransformerStageImpl::new();
        
        let series_rules = transformer.load_default_transformation_rules("series").unwrap();
        assert!(!series_rules.is_empty());
        
        let obs_rules = transformer.load_default_transformation_rules("observations").unwrap();
        assert!(!obs_rules.is_empty());
        
        let lookup_rules = transformer.load_default_transformation_rules("lookups").unwrap();
        assert!(!lookup_rules.is_empty());
        
        let unknown_rules = transformer.load_default_transformation_rules("unknown").unwrap();
        assert!(unknown_rules.is_empty());
    }

    #[test]
    fn test_transformer_stage_validation() {
        let transformer = TransformerStageImpl::new();
        let mut context = ProcessingContext::new(crate::processing::traits::ProcessingConfig::default());
        
        // Should fail with empty data readers
        assert!(transformer.validate(&context).is_err());
        
        // Add a dummy reader (this would be a real reader in practice)
        // context.data_readers.push(Box::new(DummyReader));
        // assert!(transformer.validate(&context).is_ok());
    }

    #[test]
    fn test_can_process() {
        let transformer = TransformerStageImpl::new();
        let mut context = ProcessingContext::new(crate::processing::traits::ProcessingConfig::default());
        
        // Should return false with no data readers
        assert!(!transformer.can_process(&context).unwrap());
        
        // Would return true with data readers
        // context.data_readers.push(Box::new(DummyReader));
        // assert!(transformer.can_process(&context).unwrap());
    }

    #[test]
    fn test_transformer_stats() {
        let mut transformer = TransformerStageImpl::new();
        assert_eq!(transformer.stats().records_transformed, 0);
        assert_eq!(transformer.stats().rules_applied, 0);
        
        transformer.reset_stats();
        assert_eq!(transformer.stats().records_transformed, 0);
    }

    #[test]
    fn test_dependencies() {
        let transformer = TransformerStageImpl::new();
        let deps = transformer.dependencies();
        assert_eq!(deps, vec!["loader"]);
    }

    #[test]
    fn test_cleanup() {
        let mut transformer = TransformerStageImpl::new();
        let mut context = ProcessingContext::new(crate::processing::traits::ProcessingConfig::default());
        
        // Should not fail
        assert!(transformer.cleanup(&mut context).is_ok());
    }

    #[test]
    fn test_custom_function() {
        let mut transformer = TransformerStageImpl::new();
        
        // Add a custom function
        transformer.add_custom_function("test_func".to_string(), |input| {
            Ok(format!("transformed_{}", input))
        });
        
        assert!(transformer.custom_functions.contains_key("test_func"));
    }
}