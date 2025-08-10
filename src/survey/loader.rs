//! Unified Survey Loader
//! 
//! Loads and combines survey configurations from multiple YAML files
//! into a unified structure.

use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn, info};

use crate::config::yaml_adapter::{YamlModelConfig, load_yaml_model_config};
use crate::error::{Result, Error, ConfigError};
use super::*;

/// Loader for unified survey configurations
pub struct UnifiedSurveyLoader {
    config_dir: PathBuf,
}

impl UnifiedSurveyLoader {
    /// Create a new loader with the default config directory
    pub fn new() -> Self {
        Self {
            config_dir: PathBuf::from("config/surveys"),
        }
    }
    
    /// Create a loader with a custom config directory
    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }
    
    /// Load a unified survey configuration
    pub fn load_survey(&self, survey_code: &str) -> Result<UnifiedSurvey> {
        info!("Loading unified survey configuration for {}", survey_code);
        
        let survey_dir = self.config_dir.join(survey_code.to_uppercase());
        if !survey_dir.exists() {
            return Err(Error::Config(ConfigError::LoadError {
                path: survey_dir.display().to_string(),
                source: "Survey directory not found".to_string(),
            }));
        }
        
        // Load overview.yml (required)
        let overview_path = survey_dir.join("overview.yml");
        let metadata = self.load_survey_metadata(&overview_path)?;
        
        let mut survey = UnifiedSurvey {
            metadata,
            data_model: None,
            io_config: None,
            processing_config: None,
            output_config: None,
            quality_config: None,
            runtime_config: None,
            dags_config: None,
            config_versions: HashMap::new(),
        };
        
        // Load model.yml (optional, with adapter)
        let model_path = survey_dir.join("model.yml");
        if model_path.exists() {
            match self.load_data_model(&model_path) {
                Ok(model) => {
                    survey.data_model = Some(model);
                    survey.config_versions.insert("model".to_string(), 1);
                    debug!("Loaded model.yml for {}", survey_code);
                }
                Err(e) => {
                    warn!("Failed to load model.yml for {}: {}", survey_code, e);
                }
            }
        }
        
        // Load io.yml (optional)
        let io_path = survey_dir.join("io.yml");
        if io_path.exists() {
            match self.load_io_config(&io_path) {
                Ok(config) => {
                    survey.io_config = Some(config);
                    survey.config_versions.insert("io".to_string(), 1);
                    debug!("Loaded io.yml for {}", survey_code);
                }
                Err(e) => {
                    warn!("Failed to load io.yml for {}: {}", survey_code, e);
                }
            }
        }
        
        // Load processing.yml (optional)
        let processing_path = survey_dir.join("processing.yml");
        if processing_path.exists() {
            match self.load_processing_config(&processing_path) {
                Ok(config) => {
                    survey.processing_config = Some(config);
                    survey.config_versions.insert("processing".to_string(), 1);
                    debug!("Loaded processing.yml for {}", survey_code);
                }
                Err(e) => {
                    warn!("Failed to load processing.yml for {}: {}", survey_code, e);
                }
            }
        }
        
        // Load output.yml (optional)
        let output_path = survey_dir.join("output.yml");
        if output_path.exists() {
            match self.load_output_config(&output_path) {
                Ok(config) => {
                    survey.output_config = Some(config);
                    survey.config_versions.insert("output".to_string(), 1);
                    debug!("Loaded output.yml for {}", survey_code);
                }
                Err(e) => {
                    warn!("Failed to load output.yml for {}: {}", survey_code, e);
                }
            }
        }
        
        // Load quality.yml (optional)
        let quality_path = survey_dir.join("quality.yml");
        if quality_path.exists() {
            match self.load_quality_config(&quality_path) {
                Ok(config) => {
                    survey.quality_config = Some(config);
                    survey.config_versions.insert("quality".to_string(), 1);
                    debug!("Loaded quality.yml for {}", survey_code);
                }
                Err(e) => {
                    warn!("Failed to load quality.yml for {}: {}", survey_code, e);
                }
            }
        }
        
        // Load runtime.yml (optional)
        let runtime_path = survey_dir.join("runtime.yml");
        if runtime_path.exists() {
            match self.load_runtime_config(&runtime_path) {
                Ok(config) => {
                    survey.runtime_config = Some(config);
                    survey.config_versions.insert("runtime".to_string(), 1);
                    debug!("Loaded runtime.yml for {}", survey_code);
                }
                Err(e) => {
                    warn!("Failed to load runtime.yml for {}: {}", survey_code, e);
                }
            }
        }
        
        // Load dags.yml (optional)
        let dags_path = survey_dir.join("dags.yml");
        if dags_path.exists() {
            match self.load_dags_config(&dags_path) {
                Ok(config) => {
                    survey.dags_config = Some(config);
                    survey.config_versions.insert("dags".to_string(), 1);
                    debug!("Loaded dags.yml for {}", survey_code);
                }
                Err(e) => {
                    warn!("Failed to load dags.yml for {}: {}", survey_code, e);
                }
            }
        }
        
        info!("Successfully loaded unified survey configuration for {} with configs: {:?}", 
              survey_code, survey.get_loaded_configs());
        
        Ok(survey)
    }
    
    /// Load survey metadata from overview.yml
    fn load_survey_metadata(&self, path: &Path) -> Result<SurveyMetadata> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(ConfigError::LoadError {
                path: path.display().to_string(),
                source: e.to_string(),
            }))?;
        
        // Parse the YAML with the "survey" root key
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| Error::Config(ConfigError::ParseError {
                message: format!("Failed to parse overview.yml: {}", e),
                line: None,
                column: None,
            }))?;
        
        // Extract the survey section
        let survey_value = yaml_value.get("survey")
            .ok_or_else(|| Error::Config(ConfigError::ValidationError {
                message: "Missing 'survey' section in overview.yml".to_string(),
                field: Some("survey".to_string()),
            }))?;
        
        // Deserialize the survey metadata
        let metadata: SurveyMetadata = serde_yaml::from_value(survey_value.clone())
            .map_err(|e| Error::Config(ConfigError::ParseError {
                message: format!("Failed to parse survey metadata: {}", e),
                line: None,
                column: None,
            }))?;
        
        Ok(metadata)
    }
    
    /// Load data model from model.yml using the adapter
    fn load_data_model(&self, path: &Path) -> Result<SurveyDataModel> {
        // Use the yaml_adapter to load and convert the model
        let _yaml_config = load_yaml_model_config(&path.to_path_buf())?;
        
        // Convert from adapted ModelConfig to SurveyDataModel
        // For now, we'll load the raw YAML and extract what we need
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(ConfigError::LoadError {
                path: path.display().to_string(),
                source: e.to_string(),
            }))?;
        
        let yaml_model: YamlModelConfig = serde_yaml::from_str(&content)
            .map_err(|e| Error::Config(ConfigError::ParseError {
                message: format!("Failed to parse model.yml: {}", e),
                line: None,
                column: None,
            }))?;
        
        // Convert relationships
        let relationships = yaml_model.relationships.unwrap_or_default()
            .into_iter()
            .map(|r| DataRelationship {
                from_entity: r.from.file,
                from_field: r.from.column,
                to_entity: r.to.file,
                to_field: r.to.column,
                relationship_type: r.relationship_type,
            })
            .collect();
        
        // Convert constraints
        let constraints = yaml_model.constraints.map(|c| {
            let unique_keys = c.unique.unwrap_or_default()
                .into_iter()
                .map(|u| UniqueKey {
                    entity: u.file,
                    fields: u.columns,
                })
                .collect();
            
            let required_fields = c.required_fields.unwrap_or_default()
                .into_iter()
                .map(|r| RequiredField {
                    entity: r.file,
                    fields: r.columns,
                })
                .collect();
            
            DataConstraints {
                unique_keys,
                required_fields,
            }
        });
        
        Ok(SurveyDataModel {
            series: yaml_model.series,
            data_files: yaml_model.data_files.unwrap_or_default(),
            lookups: yaml_model.lookups.unwrap_or_default(),
            relationships,
            constraints,
        })
    }
    
    /// Load I/O configuration from io.yml
    fn load_io_config(&self, path: &Path) -> Result<SurveyIoConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(ConfigError::LoadError {
                path: path.display().to_string(),
                source: e.to_string(),
            }))?;
        
        let config: SurveyIoConfig = serde_yaml::from_str(&content)
            .map_err(|e| Error::Config(ConfigError::ParseError {
                message: format!("Failed to parse io.yml: {}", e),
                line: None,
                column: None,
            }))?;
        
        Ok(config)
    }
    
    /// Load processing configuration from processing.yml
    fn load_processing_config(&self, path: &Path) -> Result<SurveyProcessingConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(ConfigError::LoadError {
                path: path.display().to_string(),
                source: e.to_string(),
            }))?;
        
        let config: SurveyProcessingConfig = serde_yaml::from_str(&content)
            .map_err(|e| Error::Config(ConfigError::ParseError {
                message: format!("Failed to parse processing.yml: {}", e),
                line: None,
                column: None,
            }))?;
        
        Ok(config)
    }
    
    /// Load output configuration from output.yml
    fn load_output_config(&self, path: &Path) -> Result<SurveyOutputConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(ConfigError::LoadError {
                path: path.display().to_string(),
                source: e.to_string(),
            }))?;
        
        let config: SurveyOutputConfig = serde_yaml::from_str(&content)
            .map_err(|e| Error::Config(ConfigError::ParseError {
                message: format!("Failed to parse output.yml: {}", e),
                line: None,
                column: None,
            }))?;
        
        Ok(config)
    }
    
    /// Load quality configuration from quality.yml
    fn load_quality_config(&self, path: &Path) -> Result<SurveyQualityConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(ConfigError::LoadError {
                path: path.display().to_string(),
                source: e.to_string(),
            }))?;
        
        let config: SurveyQualityConfig = serde_yaml::from_str(&content)
            .map_err(|e| Error::Config(ConfigError::ParseError {
                message: format!("Failed to parse quality.yml: {}", e),
                line: None,
                column: None,
            }))?;
        
        Ok(config)
    }
    
    /// Load runtime configuration from runtime.yml
    fn load_runtime_config(&self, path: &Path) -> Result<SurveyRuntimeConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(ConfigError::LoadError {
                path: path.display().to_string(),
                source: e.to_string(),
            }))?;
        
        let config: SurveyRuntimeConfig = serde_yaml::from_str(&content)
            .map_err(|e| Error::Config(ConfigError::ParseError {
                message: format!("Failed to parse runtime.yml: {}", e),
                line: None,
                column: None,
            }))?;
        
        Ok(config)
    }
    
    /// Load DAGs configuration from dags.yml
    fn load_dags_config(&self, path: &Path) -> Result<SurveyDagsConfig> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::Config(ConfigError::LoadError {
                path: path.display().to_string(),
                source: e.to_string(),
            }))?;
        
        let config: SurveyDagsConfig = serde_yaml::from_str(&content)
            .map_err(|e| Error::Config(ConfigError::ParseError {
                message: format!("Failed to parse dags.yml: {}", e),
                line: None,
                column: None,
            }))?;
        
        Ok(config)
    }
    
    /// List all available surveys
    pub fn list_surveys(&self) -> Result<Vec<String>> {
        let mut surveys = Vec::new();
        
        let entries = fs::read_dir(&self.config_dir)
            .map_err(|e| Error::Config(ConfigError::LoadError {
                path: self.config_dir.display().to_string(),
                source: e.to_string(),
            }))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| Error::Config(ConfigError::LoadError {
                path: "directory entry".to_string(),
                source: e.to_string(),
            }))?;
            
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Skip special directories
                    if !name.starts_with('_') && !name.starts_with('.') && name != "TEMPLATE" {
                        // Check if overview.yml exists
                        if path.join("overview.yml").exists() {
                            surveys.push(name.to_string());
                        }
                    }
                }
            }
        }
        
        surveys.sort();
        Ok(surveys)
    }
}

impl Default for UnifiedSurveyLoader {
    fn default() -> Self {
        Self::new()
    }
}