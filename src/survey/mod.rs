//! Unified Survey Module
//! 
//! This module provides a unified representation of BLS survey data,
//! combining information from multiple YAML configuration files into
//! a single, cohesive structure.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::yaml_adapter::{SeriesDefinition, DataFileDefinition, LookupDefinition};

pub mod loader;

/// Unified survey representation combining all configuration aspects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSurvey {
    /// Survey metadata from overview.yml
    pub metadata: SurveyMetadata,
    
    /// Data model from model.yml (adapted)
    pub data_model: Option<SurveyDataModel>,
    
    /// Input/Output configuration from io.yml
    pub io_config: Option<SurveyIoConfig>,
    
    /// Processing configuration from processing.yml
    pub processing_config: Option<SurveyProcessingConfig>,
    
    /// Output configuration from output.yml
    pub output_config: Option<SurveyOutputConfig>,
    
    /// Quality configuration from quality.yml
    pub quality_config: Option<SurveyQualityConfig>,
    
    /// Runtime configuration from runtime.yml
    pub runtime_config: Option<SurveyRuntimeConfig>,
    
    /// DAGs configuration from dags.yml
    pub dags_config: Option<SurveyDagsConfig>,
    
    /// Configuration version tracking
    pub config_versions: HashMap<String, u32>,
}

/// Survey metadata from overview.yml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyMetadata {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub size_class: Option<String>,
    pub characteristics: Option<SurveyCharacteristics>,
    pub contacts: Option<Vec<Contact>>,
    pub documentation: Option<Documentation>,
    pub tags: Option<Vec<String>>,
    pub definition: Option<String>,
}

/// Survey characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyCharacteristics {
    pub frequency: Option<String>,
    pub coverage: Option<String>,
    pub time_series_count: Option<String>,
    pub classification_system: Option<String>,
    pub update_schedule: Option<String>,
    pub begin_year: Option<u32>,
    pub survey_methodology: Option<String>,
    pub calculation_basis: Option<String>,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub role: String,
    pub name: String,
    pub email: Option<String>,
}

/// Documentation references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Documentation {
    pub primary: Option<String>,
    pub data_dictionary: Option<String>,
    pub notes: Option<String>,
}

/// Survey data model (adapted from model.yml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyDataModel {
    pub series: Option<SeriesDefinition>,
    pub data_files: Vec<DataFileDefinition>,
    pub lookups: Vec<LookupDefinition>,
    pub relationships: Vec<DataRelationship>,
    pub constraints: Option<DataConstraints>,
}

/// Data relationship definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRelationship {
    pub from_entity: String,
    pub from_field: String,
    pub to_entity: String,
    pub to_field: String,
    pub relationship_type: String,
}

/// Data constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConstraints {
    pub unique_keys: Vec<UniqueKey>,
    pub required_fields: Vec<RequiredField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueKey {
    pub entity: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredField {
    pub entity: String,
    pub fields: Vec<String>,
}

/// Survey I/O configuration (from io.yml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyIoConfig {
    pub discovery: Option<DiscoveryConfig>,
    pub combining: Option<CombiningConfig>,
    pub partition_hints: Option<PartitionConfig>,
    pub mmap: Option<MmapConfig>,
    pub lookup_strategy: Option<LookupStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub root: PathBuf,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombiningConfig {
    pub enabled: bool,
    pub strategy: String,
    pub thresholds: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionConfig {
    pub strategy: String,
    pub max_partition_size_mb: Option<u32>,
    pub partition_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmapConfig {
    pub enable_for_mb_greater_than: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupStrategy {
    pub preload: Vec<String>,
    pub lazy: Vec<String>,
    pub mmap: Vec<String>,
    pub cache_all: bool,
}

/// Survey processing configuration (from processing.yml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyProcessingConfig {
    pub strategy: ProcessingStrategyConfig,
    pub resources: Option<ResourceConfig>,
    pub transformations: Option<Vec<TransformationConfig>>,
    pub validation: Option<ValidationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStrategyConfig {
    pub mode: String,
    pub batch_size: Option<u32>,
    pub parallel_tasks: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub memory_limit_gb: Option<f64>,
    pub cpu_cores: Option<u32>,
    pub disk_space_gb: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformationConfig {
    pub name: String,
    pub enabled: bool,
    pub params: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub enabled: bool,
    pub rules: Vec<String>,
    pub fail_on_error: bool,
}

/// Survey output configuration (from output.yml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyOutputConfig {
    pub format: String,
    pub destination: DestinationConfig,
    pub partitioning: Option<OutputPartitioningConfig>,
    pub compression: Option<CompressionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationConfig {
    pub base_path: PathBuf,
    pub structure: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPartitioningConfig {
    pub enabled: bool,
    pub by: Vec<String>,
    pub max_size_mb: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub algorithm: String,
    pub level: Option<u32>,
}

/// Survey quality configuration (from quality.yml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyQualityConfig {
    pub policy: QualityPolicy,
    pub checks: Vec<QualityCheck>,
    pub thresholds: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityPolicy {
    pub mode: String,
    pub report_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityCheck {
    pub name: String,
    pub check_type: String,
    pub enabled: bool,
    pub params: Option<HashMap<String, serde_yaml::Value>>,
}

/// Survey runtime configuration (from runtime.yml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyRuntimeConfig {
    pub logging: LoggingConfig,
    pub monitoring: Option<MonitoringConfig>,
    pub feature_flags: Option<HashMap<String, bool>>,
    pub retry_policy: Option<RetryPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub metrics: MetricsConfig,
    pub tracing: Option<TracingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub export_interval_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    pub enabled: bool,
    pub sample_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff_seconds: u32,
    pub exponential: bool,
}

/// Survey DAGs configuration (from dags.yml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurveyDagsConfig {
    pub pipelines: Vec<PipelineConfig>,
    pub dependencies: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub name: String,
    pub description: Option<String>,
    pub stages: Vec<StageConfig>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageConfig {
    pub name: String,
    pub stage_type: String,
    pub params: Option<HashMap<String, serde_yaml::Value>>,
    pub depends_on: Option<Vec<String>>,
}

impl UnifiedSurvey {
    /// Create a new unified survey with minimal required metadata
    pub fn new(code: String, name: String) -> Self {
        Self {
            metadata: SurveyMetadata {
                code,
                name,
                description: None,
                size_class: None,
                characteristics: None,
                contacts: None,
                documentation: None,
                tags: None,
                definition: None,
            },
            data_model: None,
            io_config: None,
            processing_config: None,
            output_config: None,
            quality_config: None,
            runtime_config: None,
            dags_config: None,
            config_versions: HashMap::new(),
        }
    }
    
    /// Check if the survey has the minimum required configuration
    pub fn is_valid(&self) -> bool {
        !self.metadata.code.is_empty() && !self.metadata.name.is_empty()
    }
    
    /// Get a summary of what configurations are loaded
    pub fn get_loaded_configs(&self) -> Vec<String> {
        let mut configs = vec!["overview".to_string()];
        
        if self.data_model.is_some() {
            configs.push("model".to_string());
        }
        if self.io_config.is_some() {
            configs.push("io".to_string());
        }
        if self.processing_config.is_some() {
            configs.push("processing".to_string());
        }
        if self.output_config.is_some() {
            configs.push("output".to_string());
        }
        if self.quality_config.is_some() {
            configs.push("quality".to_string());
        }
        if self.runtime_config.is_some() {
            configs.push("runtime".to_string());
        }
        if self.dags_config.is_some() {
            configs.push("dags".to_string());
        }
        
        configs
    }
}