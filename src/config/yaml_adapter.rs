//! YAML Configuration Adapter
//! 
//! This module provides adapters to bridge the gap between the actual YAML
//! configuration structure used in config/surveys/* and the Rust configuration
//! models expected by the system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::model::{ModelConfig, DataModel, FieldDefinition, IndexDefinition, IoConfig, InputConfig, IoOutputConfig, FileFormatConfig, InputValidationConfig, NamingConfig, ProcessingConfig, ProcessingStrategy, MmapConfig, ChunkingConfig, DagsConfig, DagDefinition, TaskDefinition, RetryConfig, BackoffStrategy, SlaConfig};
use crate::error::Result;

/// The actual structure found in model.yml files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlModelConfig {
    pub config_version: u32,
    
    /// Series definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<SeriesDefinition>,
    
    /// Data files definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_files: Option<Vec<DataFileDefinition>>,
    
    /// Lookup tables definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookups: Option<Vec<LookupDefinition>>,
    
    /// Relationships between data entities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationships: Option<Vec<RelationshipDefinition>>,
    
    /// Data constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<ConstraintsDefinition>,
    
    /// Custom type definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<HashMap<String, TypeDefinition>>,
}

/// Series definition from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesDefinition {
    pub path: String,
    pub index: String,
    pub estimated_size_gb: f64,
    pub schema: HashMap<String, FieldSchema>,
}

/// Data file definition from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFileDefinition {
    pub id: String,
    pub pattern: String,
    pub index: String,
    pub estimated_size_gb: f64,
    pub schema: HashMap<String, FieldSchema>,
}

/// Lookup table definition from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupDefinition {
    pub id: String,
    pub path: String,
    pub index: String,
    pub description: String,
    pub schema: HashMap<String, FieldSchema>,
}

/// Field schema from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    #[serde(rename = "type")]
    pub field_type: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_value: Option<serde_yaml::Value>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
}

/// Relationship definition from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipDefinition {
    pub from: RelationshipEndpoint,
    pub to: RelationshipEndpoint,
    
    #[serde(rename = "type")]
    pub relationship_type: String,
}

/// Relationship endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEndpoint {
    pub file: String,
    pub column: String,
}

/// Constraints definition from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintsDefinition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<Vec<ConstraintItem>>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_fields: Option<Vec<ConstraintItem>>,
}

/// Constraint item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintItem {
    pub file: String,
    pub columns: Vec<String>,
}

/// Type definition from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub base: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u32>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
}

impl YamlModelConfig {
    /// Convert YAML model config to the expected ModelConfig structure
    pub fn to_model_config(&self) -> ModelConfig {
        let mut models = HashMap::new();
        
        // Convert series to a DataModel
        if let Some(series) = &self.series {
            let fields = series.schema.iter().map(|(name, schema)| {
                FieldDefinition {
                    name: name.clone(),
                    field_type: schema.field_type.clone(),
                    required: !schema.nullable.unwrap_or(true),
                    description: None,
                    validation: Vec::new(),
                    default: schema.example_value.clone(),
                }
            }).collect();
            
            let model = DataModel {
                name: "series".to_string(),
                description: Some("Series data model".to_string()),
                fields,
                primary_key: vec![series.index.clone()],
                indexes: vec![
                    IndexDefinition {
                        name: "primary".to_string(),
                        fields: vec![series.index.clone()],
                        unique: true,
                    }
                ],
            };
            
            models.insert("series".to_string(), model);
        }
        
        // Convert data files to DataModels
        if let Some(data_files) = &self.data_files {
            for file_def in data_files {
                let fields = file_def.schema.iter().map(|(name, schema)| {
                    FieldDefinition {
                        name: name.clone(),
                        field_type: schema.field_type.clone(),
                        required: !schema.nullable.unwrap_or(true),
                        description: None,
                        validation: Vec::new(),
                        default: schema.example_value.clone(),
                    }
                }).collect();
                
                let model = DataModel {
                    name: file_def.id.clone(),
                    description: Some(format!("Data file: {}", file_def.pattern)),
                    fields,
                    primary_key: vec![file_def.index.clone()],
                    indexes: vec![
                        IndexDefinition {
                            name: "primary".to_string(),
                            fields: vec![file_def.index.clone()],
                            unique: false,
                        }
                    ],
                };
                
                models.insert(file_def.id.clone(), model);
            }
        }
        
        // Convert lookups to DataModels
        if let Some(lookups) = &self.lookups {
            for lookup_def in lookups {
                let fields = lookup_def.schema.iter().map(|(name, schema)| {
                    FieldDefinition {
                        name: name.clone(),
                        field_type: schema.field_type.clone(),
                        required: !schema.nullable.unwrap_or(true),
                        description: None,
                        validation: Vec::new(),
                        default: schema.example_value.clone(),
                    }
                }).collect();
                
                let model = DataModel {
                    name: lookup_def.id.clone(),
                    description: Some(lookup_def.description.clone()),
                    fields,
                    primary_key: vec![lookup_def.index.clone()],
                    indexes: vec![
                        IndexDefinition {
                            name: "primary".to_string(),
                            fields: vec![lookup_def.index.clone()],
                            unique: true,
                        }
                    ],
                };
                
                models.insert(lookup_def.id.clone(), model);
            }
        }
        
        ModelConfig {
            config_version: self.config_version,
            models,
        }
    }
}

/// Load and adapt a YAML model configuration file
pub fn load_yaml_model_config(path: &PathBuf) -> Result<ModelConfig> {
    use std::fs;
    use crate::error::{Error, ConfigError};
    
    let content = fs::read_to_string(path)
        .map_err(|e| Error::Config(ConfigError::LoadError {
            path: path.display().to_string(),
            source: e.to_string(),
        }))?;
    
    let yaml_config: YamlModelConfig = serde_yaml::from_str(&content)
        .map_err(|e| Error::Config(ConfigError::ParseError {
            message: format!("Failed to parse YAML model config: {}", e),
            line: None,
            column: None,
        }))?;
    
    Ok(yaml_config.to_model_config())
}

/// Extract lookup table ids from a YAML model configuration file
pub fn load_yaml_lookup_ids(path: &PathBuf) -> Result<Vec<String>> {
    use std::fs;
    use crate::error::{Error, ConfigError};

    let content = fs::read_to_string(path)
        .map_err(|e| Error::Config(ConfigError::LoadError {
            path: path.display().to_string(),
            source: e.to_string(),
        }))?;

    let yaml_config: YamlModelConfig = serde_yaml::from_str(&content)
        .map_err(|e| Error::Config(ConfigError::ParseError {
            message: format!("Failed to parse YAML model config: {}", e),
            line: None,
            column: None,
        }))?;

    let mut ids: Vec<String> = Vec::new();
    if let Some(lookups) = yaml_config.lookups {
        for l in lookups {
            ids.push(l.id);
        }
    }
    Ok(ids)
}

/// The actual structure found in io.yml files (BLS style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlIoConfig {
    pub config_version: u32,
    
    /// Discovery configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery: Option<DiscoveryConfig>,
    
    /// Combining configuration  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub combining: Option<HashMap<String, serde_yaml::Value>>,
    
    /// Partition hints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_hints: Option<HashMap<String, serde_yaml::Value>>,
    
    /// Memory mapping configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mmap: Option<HashMap<String, serde_yaml::Value>>,
    
    /// Lookup strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lookup_strategy: Option<HashMap<String, serde_yaml::Value>>,
}

/// Discovery configuration from io.yml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub root: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl YamlIoConfig {
    /// Convert to the expected IoConfig structure
    pub fn to_io_config(&self) -> IoConfig {
        let (base_dir, patterns) = if let Some(discovery) = &self.discovery {
            let root = PathBuf::from(&discovery.root);
            
            // Map BLS-style patterns to actual file paths based on directory structure
            let mut mapped_patterns = Vec::new();
            for pattern in &discovery.include {
                if pattern.contains(".series") {
                    // Series files are at root level
                    mapped_patterns.push(pattern.clone());
                } else if pattern.contains(".data") {
                    // Data files are in data/ subdirectory
                    mapped_patterns.push(format!("data/{}", pattern));
                } else if pattern.contains(".area") || pattern.contains(".item") || 
                         pattern.contains(".footnote") || pattern.contains(".period") ||
                         pattern.contains(".contacts") || pattern.contains(".seasonal") {
                    // Lookup files are in map/ subdirectory
                    mapped_patterns.push(format!("map/{}", pattern));
                } else {
                    // Keep other patterns as-is
                    mapped_patterns.push(pattern.clone());
                }
            }
            
            (root, mapped_patterns)
        } else {
            // Fallback defaults
            (PathBuf::from("data/raw/bls"), vec!["*.series".to_string(), "*.data.*".to_string()])
        };
        
        IoConfig {
            config_version: self.config_version,
            input: InputConfig {
                base_dir,
                patterns,
                format: FileFormatConfig::default(),
                validation: InputValidationConfig::default(),
            },
            output: IoOutputConfig {
                base_dir: PathBuf::from("data/processed"),
                structure: vec!["survey".to_string()],
                naming: NamingConfig::default(),
            },
        }
    }
}

/// Load and adapt a YAML io configuration file
pub fn load_yaml_io_config(path: &PathBuf) -> Result<IoConfig> {
    use std::fs;
    use crate::error::{Error, ConfigError};
    
    let content = fs::read_to_string(path)
        .map_err(|e| Error::Config(ConfigError::LoadError {
            path: path.display().to_string(),
            source: e.to_string(),
        }))?;
    
    let yaml_config: YamlIoConfig = serde_yaml::from_str(&content)
        .map_err(|e| Error::Config(ConfigError::ParseError {
            message: format!("Failed to parse YAML io config: {}", e),
            line: None,
            column: None,
        }))?;
    
    Ok(yaml_config.to_io_config())
}

// -------------------------
// Processing YAML Adapter
// -------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlProcessingConfig {
    pub config_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub strategy: Option<YamlProcessingStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")] pub resources: Option<YamlProcessingResources>,
    #[serde(skip_serializing_if = "Option::is_none")] pub io: Option<YamlProcessingIo>,
    #[serde(skip_serializing_if = "Option::is_none")] pub features: Option<YamlProcessingFeatures>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlProcessingStrategy {
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub max_concurrency: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlProcessingResources {
    #[serde(skip_serializing_if = "Option::is_none")] pub max_threads: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub memory_limit_gb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlProcessingIo {
    #[serde(skip_serializing_if = "Option::is_none")] pub chunk_size_mb: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")] pub buffer_pool_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlProcessingFeatures {
    #[serde(skip_serializing_if = "Option::is_none")] pub use_mmap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub enable_statistics: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")] pub enable_profiling: Option<bool>,
}

impl YamlProcessingConfig {
    pub fn to_processing_config(&self) -> ProcessingConfig {
        let mut cfg = ProcessingConfig::default();
        if let Some(v) = self.config_version { cfg.config_version = v; }
        if let Some(strategy) = &self.strategy {
            if let Some(mode) = &strategy.mode {
                cfg.strategy = match mode.as_str() {
                    "in_memory" => ProcessingStrategy::InMemory,
                    "chunked" => ProcessingStrategy::Chunked,
                    "mmap" => ProcessingStrategy::Mmap,
                    _ => cfg.strategy,
                };
            }
            if let Some(maxc) = strategy.max_concurrency {
                cfg.parallel = maxc > 1;
                if cfg.max_threads == ProcessingConfig::default().max_threads && maxc > 0 {
                    cfg.max_threads = maxc;
                }
            }
        }
        if let Some(resources) = &self.resources {
            if let Some(t) = resources.max_threads { cfg.max_threads = t; }
            if let Some(_gb) = resources.memory_limit_gb { /* optional */ }
        }
        if let Some(_io) = &self.io { /* optional mapping not required for validation */ }
        if let Some(features) = &self.features {
            if let Some(use_mmap) = features.use_mmap { cfg.mmap.enabled = use_mmap; }
        }
        cfg
    }
}

pub fn load_yaml_processing_config(path: &PathBuf) -> Result<ProcessingConfig> {
    use std::fs;
    use crate::error::{Error, ConfigError};

    let content = fs::read_to_string(path)
        .map_err(|e| Error::Config(ConfigError::LoadError { path: path.display().to_string(), source: e.to_string() }))?;
    let yaml_config: YamlProcessingConfig = serde_yaml::from_str(&content)
        .map_err(|e| Error::Config(ConfigError::ParseError { message: format!("Failed to parse YAML processing config: {}", e), line: None, column: None }))?;
    Ok(yaml_config.to_processing_config())
}

// ---------------------
// DAGs YAML Adapter
// ---------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDagsConfig {
    pub config_version: Option<u32>,
    pub graph: YamlDagGraph,
    #[serde(skip_serializing_if = "Option::is_none")] pub settings: Option<YamlDagSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDagGraph {
    pub tasks: Vec<YamlDagTask>,
    #[serde(skip_serializing_if = "Option::is_none")] pub edges: Option<Vec<YamlDagEdge>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDagTask {
    pub id: String,
    #[serde(rename = "type")] pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub inputs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")] pub retries: Option<YamlDagRetries>,
    #[serde(skip_serializing_if = "Option::is_none")] pub sla_minutes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub parallelism: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDagRetries { pub max: Option<u32>, pub backoff_sec: Option<u64> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDagEdge { pub from: String, pub to: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YamlDagSettings {
    #[serde(skip_serializing_if = "Option::is_none")] pub retries: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub retry_delay_minutes: Option<u64>,
}

impl YamlDagsConfig {
    pub fn to_dags_config(&self) -> DagsConfig {
        let mut tasks_map: HashMap<String, TaskDefinition> = HashMap::new();
        for t in &self.graph.tasks {
            let mut retry = RetryConfig { max_attempts: 0, backoff: BackoffStrategy::Fixed, delay: 0, initial_delay: None };
            if let Some(r) = &t.retries {
                if let Some(m) = r.max { retry.max_attempts = m; }
                if let Some(d) = r.backoff_sec { retry.delay = d; }
            }
            let sla = t.sla_minutes.map(|m| SlaConfig { max_duration: m, alert_on_failure: true });
            let depends_on = t.inputs.clone().unwrap_or_default();
            let task = TaskDefinition {
                name: t.id.clone(),
                task_type: t.kind.clone(),
                depends_on,
                retry,
                sla,
                parameters: HashMap::new(),
                description: t.description.clone(),
                stage: t.kind.clone(),
                timeout: None,
            };
            tasks_map.insert(t.id.clone(), task);
        }
        if let Some(edges) = &self.graph.edges {
            for e in edges {
                if let Some(to_task) = tasks_map.get_mut(&e.to) {
                    if !to_task.depends_on.contains(&e.from) {
                        to_task.depends_on.push(e.from.clone());
                    }
                }
            }
        }
        let defaults = crate::config::model::TaskDefaults::default();
        let dag_def = DagDefinition { name: "pipeline".to_string(), description: None, tasks: tasks_map, defaults };
        let mut dags_map = HashMap::new();
        dags_map.insert("pipeline".to_string(), dag_def);
        DagsConfig { config_version: self.config_version.unwrap_or(1), dags: dags_map }
    }
}

pub fn load_yaml_dags_config(path: &PathBuf) -> Result<DagsConfig> {
    use std::fs;
    use crate::error::{Error, ConfigError};

    let content = fs::read_to_string(path)
        .map_err(|e| Error::Config(ConfigError::LoadError { path: path.display().to_string(), source: e.to_string() }))?;
    let yaml_config: YamlDagsConfig = serde_yaml::from_str(&content)
        .map_err(|e| Error::Config(ConfigError::ParseError { message: format!("Failed to parse YAML dags config: {}", e), line: None, column: None }))?;
    Ok(yaml_config.to_dags_config())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_yaml_lookup_ids_ap() {
        let path = PathBuf::from("config/surveys/AP/model.yml");
        let ids = load_yaml_lookup_ids(&path).expect("failed to load lookup ids");
        assert_eq!(ids, vec![
            "area".to_string(),
            "footnote".to_string(),
            "item".to_string(),
            "period".to_string(),
        ]);
    }
}
