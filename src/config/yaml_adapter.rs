//! YAML Configuration Adapter
//! 
//! This module provides adapters to bridge the gap between the actual YAML
//! configuration structure used in config/surveys/* and the Rust configuration
//! models expected by the system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::model::{ModelConfig, DataModel, FieldDefinition, IndexDefinition};
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