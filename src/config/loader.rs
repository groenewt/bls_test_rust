//! # Configuration Loader
//!
//! This module provides configuration loading functionality for the Rusty BLS Data Processing system.
//! It supports loading modular, layered configurations with proper precedence rules.
//!
//! ## Layered Configuration Structure
//!
//! Configurations are loaded with the following precedence (last wins):
//! 1. Shared base configurations (`_shared/base_*.yml`)
//! 2. Survey-specific configurations (`<survey>/*.yml`)
//! 3. Default overrides (`<survey>/overrides/defaults.yml`)
//! 4. Environment overrides (`<survey>/overrides/env/<env>.yml`)
//! 5. Local overrides (`<survey>/overrides/local.yml`)
//!
//! ## Usage

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::de::DeserializeOwned;
use serde_yaml::Value;
use tracing::{info, warn, debug, error};
use crate::config::model::{SurveyConfig, OverviewConfig, ModelConfig, IoConfig, ProcessingConfig, OutputConfig, QualityConfig, RuntimeConfig, DagsConfig, OverrideConfig, Config};
use crate::error::{Result, ConfigError};
use crate::utils::{file, path};

/// Configuration source enumeration
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Load from a file path
    File(PathBuf),
    /// Load from a string content
    String(String, ConfigFormat),
    /// Load from environment variables
    Environment,
    /// Load default configuration for a survey
    Default(String),
}

/// Configuration format enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigFormat {
    /// YAML format
    Yaml,
    /// JSON format
    Json,
    /// Auto-detect from file extension
    Auto,
}

/// Configuration loader
pub struct ConfigLoader {
    /// Base directory for configuration files
    config_dir: PathBuf,
}

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new() -> Self {
        let path_utils = path::PathUtils::new();
        Self {
            config_dir: path_utils.config_dir(),
        }
    }

    /// Create a configuration loader with a custom config directory
    pub fn with_config_dir<P: AsRef<Path>>(config_dir: P) -> Self {
        Self {
            config_dir: config_dir.as_ref().to_path_buf(),
        }
    }

    /// Load configuration from a file
    pub fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<Config> {
        let path = path.as_ref();
        
        // Validate the path
        let path_utils = path::PathUtils::new();
        path_utils.validate_path(path)
            .map_err(|e| ConfigError::LoadError { 
                path: path.display().to_string(), 
                source: format!("Invalid path: {}", e) 
            })?;

        // Resolve relative paths
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config_dir.join(path)
        };

        // Check if file exists
        if !full_path.exists() {
            return Err(ConfigError::FileNotFound(full_path.display().to_string()));
        }

        // Determine format from file extension
        let format = self.detect_format(&full_path)?;

        // Read file content
        let content = file::read_to_string(&full_path)
            .map_err(|e| ConfigError::InvalidArgument(format!("Failed to read file: {}", e)))?;

        // Parse configuration
        self.parse_config(&content, format)
    }

    /// Load configuration for a specific survey with layered precedence
    pub fn load_survey_config(&self, survey_code: &str, environment: Option<&str>) -> Result<SurveyConfig> {
        let survey_code = survey_code.to_uppercase();
        let env = environment.unwrap_or("dev");
        
        info!("Loading survey config for {} in {} environment", survey_code, env);
        
        // Check if modular directory structure exists
        let survey_dir = path::survey_dir(&survey_code);
        let survey_path = self.config_dir.join(&survey_dir);
        
        if survey_path.exists() {
            self.load_modular_config(&survey_code, env)
        } else {
            // Fall back to monolithic config for backward compatibility
            warn!("Modular config directory not found for {}, falling back to monolithic config", survey_code);
            self.load_legacy_config(&survey_code)
        }
    }

    /// Load configuration for a specific survey (legacy method for backward compatibility)
    pub fn load_for_survey(&self, survey_code: &str) -> Result<Config> {
        // Validate survey code
        crate::utils::validation::validate_survey_code(survey_code)
            .map_err(|e| ConfigError::InvalidArgument(format!("Invalid survey code: {}", e)))?;

        // Try different file extensions
        let survey_code_lower = survey_code.to_lowercase();
        let possible_files = vec![
            format!("surveys/{}.yml", survey_code_lower),
            format!("surveys/{}.yaml", survey_code_lower),
            format!("surveys/{}.json", survey_code_lower),
        ];

        for file_path in possible_files {
            let full_path = self.config_dir.join(&file_path);
            if full_path.exists() {
                return self.load_from_file(&full_path);
            }
        }

        // If no configuration file found, create a default configuration
        tracing::warn!("No configuration file found for survey '{}', using default configuration", survey_code);
        Ok(Config::new(survey_code))
    }

    /// Load configuration from string content
    pub fn load_from_string(&self, content: &str, format: ConfigFormat) -> Result<Config> {
        self.parse_config(content, format)
    }

    /// Load configuration from environment variables
    pub fn load_from_environment(&self, survey_code: &str) -> Result<Config> {
        // Create base configuration
        let mut config = Config::new(survey_code);

        // Override with environment variables
        if let Ok(strategy) = std::env::var("RUSTY_PROCESSING_STRATEGY") {
            config.processing.strategy = match strategy.as_str() {
                "in_memory" => crate::config::model::ProcessingStrategy::InMemory,
                "chunked" => crate::config::model::ProcessingStrategy::Chunked,
                "mmap" => crate::config::model::ProcessingStrategy::Mmap,
                _ => return Err(ConfigError::ValidationError { 
                    message: format!("Invalid processing strategy: {}", strategy),
                    field: Some("strategy".to_string()),
                }.into()),
            };
        }

        if let Ok(threads) = std::env::var("RUSTY_MAX_THREADS") {
            config.processing.max_threads = threads.parse()
                .map_err(|e| ConfigError::ValidationError { 
                    message: format!("Invalid max_threads: {}", e),
                    field: Some("max_threads".to_string()),
                })?;
        }

        if let Ok(chunk_size) = std::env::var("RUSTY_CHUNK_SIZE") {
            config.processing.chunk_size = chunk_size.parse()
                .map_err(|e| ConfigError::MergeError(format!("Invalid chunk_size: {}", e)))?;
        }

        if let Ok(output_dir) = std::env::var("RUSTY_OUTPUT_DIR") {
            config.output.output_dir = output_dir;
        }

            //    if let Ok(on_error) = std::env::var("RUSTY_ON_ERROR") {
 //        config.error_handling.on_error = on_error;
 //    }

            //    if let Ok(max_errors) = std::env::var("RUSTY_MAX_ERRORS") {
 //        config.error_handling.max_errors = max_errors.parse()
                //            .map_err(|e| ConfigError::ValidationError { message: format!("Invalid max_errors: {}", e), field: Some("max_errors".to_string()) })?;
 //    }

        Ok(config)
    }

    /// Load modular configuration with layered precedence
    fn load_modular_config(&self, survey_code: &str, environment: &str) -> Result<SurveyConfig> {
        let mut config = SurveyConfig::new(survey_code);
        
        // Layer 1: Load shared base configurations
        self.load_shared_configs(&mut config)?;
        
        // Layer 2: Load survey-specific configurations
        self.load_survey_configs(&mut config, survey_code)?;
        
        // Layer 3: Load overrides with precedence: defaults → env → local
        self.load_override_configs(&mut config, survey_code, environment)?;
        
        // Update metadata after all merges
        config.update();
        
        Ok(config)
    }
    
    /// Load legacy monolithic configuration and map to SurveyConfig
    fn load_legacy_config(&self, survey_code: &str) -> Result<SurveyConfig> {
        // Try to load legacy monolithic config
        let survey_code_lower = survey_code.to_lowercase();
        let possible_files = vec![
            format!("surveys/{}.yml", survey_code_lower),
            format!("surveys/{}.yaml", survey_code_lower),
            format!("surveys/{}.json", survey_code_lower),
        ];

        for file_path in possible_files {
            let full_path = self.config_dir.join(&file_path);
            if full_path.exists() {
                warn!("Loading legacy monolithic config from {}", file_path);
                // TODO: Implement legacy config mapping to SurveyConfig
                return Ok(SurveyConfig::new(survey_code));
            }
        }

        // If no configuration file found, create a default configuration
        warn!("No configuration file found for survey '{}', using default configuration", survey_code);
        Ok(SurveyConfig::new(survey_code))
    }
    
    /// Load shared base configurations
    fn load_shared_configs(&self, config: &mut SurveyConfig) -> Result<()> {
        let shared_dir = self.config_dir.join("surveys/_shared");
        
        // Load base processing config if it exists
        let base_processing_path = shared_dir.join("base_processing.yml");
        if base_processing_path.exists() {
            if let Ok(base_processing) = file::read_yaml(&base_processing_path) {
                config.merge_processing_config(&base_processing)
                    .map_err(|e| ConfigError::MergeError(format!("Failed to merge base processing config: {}", e)))?;
                debug!("Merged shared base processing config");
            }
        }
        
        // Load other shared base configs as needed
        // TODO: Add more shared base configs (base_output.yml, base_quality.yml, etc.)
        
        Ok(())
    }
    
    /// Load survey-specific configurations
    fn load_survey_configs(&self, config: &mut SurveyConfig, survey_code: &str) -> Result<()> {
        let survey_dir = self.config_dir.join(path::survey_dir(survey_code));
        
        // Load overview.yml (required)
        let overview_path = survey_dir.join("overview.yml");
        if overview_path.exists() {
            let overview_config = file::read_yaml::<OverviewConfig, _>(&overview_path)
                .map_err(|e| ConfigError::LoadError { 
                    path: overview_path.display().to_string(), 
                    source: format!("Failed to load overview.yml: {}", e) 
                })?;
            config.merge_overview_config(&overview_config)
                .map_err(|e| ConfigError::MergeError(format!("Failed to merge overview config: {}", e)))?;
            debug!("Loaded overview.yml for {}", survey_code);
        } else {
            return Err(ConfigError::MissingFile(format!("Required file overview.yml not found for survey {}", survey_code)));
        }
        
        // Load model.yml (required)
        let model_path = survey_dir.join("model.yml");
        if model_path.exists() {
            let model_config = file::read_yaml::<ModelConfig, _>(&model_path)
                .map_err(|e| ConfigError::LoadError {
                    path: model_path.display().to_string(),
                    source: format!("Failed to load model.yml: {}", e),
                })?;
            config.merge_model_config(&model_config)
                .map_err(|e| ConfigError::MergeError(format!("Failed to merge model config: {}", e)))?;
            debug!("Loaded model.yml for {}", survey_code);
        } else {
            return Err(ConfigError::MissingFile(format!("Required file model.yml not found for survey {}", survey_code)));
        }
        
        // Load other required configs
        self.load_optional_survey_config(config, &survey_dir, "io.yml", |c, cfg: IoConfig| c.merge_io_config(&cfg).map_err(|e| ConfigError::MergeError(e).into()))?;
        self.load_optional_survey_config(config, &survey_dir, "processing.yml", |c, cfg: ProcessingConfig| c.merge_processing_config(&cfg).map_err(|e| ConfigError::MergeError(e).into()))?;
        self.load_optional_survey_config(config, &survey_dir, "output.yml", |c, cfg: OutputConfig| c.merge_output_config(&cfg).map_err(|e| ConfigError::MergeError(e).into()))?;
        self.load_optional_survey_config(config, &survey_dir, "quality.yml", |c, cfg: QualityConfig| c.merge_quality_config(&cfg).map_err(|e| ConfigError::MergeError(e).into()))?;
        self.load_optional_survey_config(config, &survey_dir, "runtime.yml", |c, cfg: RuntimeConfig| c.merge_runtime_config(&cfg).map_err(|e| ConfigError::MergeError(e).into()))?;
        
        // Load optional DAGs config
        let dags_path = survey_dir.join("dags.yml");
        if dags_path.exists() {
            let dags_config = file::read_yaml::<DagsConfig, _>(&dags_path)
                .map_err(|e| ConfigError::LoadError {
                    path: dags_path.display().to_string(),
                    source: format!("Failed to load dags.yml: {}", e),
                })?;
            config.dags = Some(dags_config);
            debug!("Loaded dags.yml for {}", survey_code);
        }
        
        Ok(())
    }
    
    /// Load optional survey configuration file
    fn load_optional_survey_config<T, F>(&self, config: &mut SurveyConfig, survey_dir: &Path, filename: &str, merge_fn: F) -> Result<()>
    where
        T: DeserializeOwned,
        F: FnOnce(&mut SurveyConfig, T) -> Result<()>,
    {
        let config_path = survey_dir.join(filename);
        if config_path.exists() {
            let cfg = file::read_yaml::<T, _>(&config_path)
                .map_err(|e| ConfigError::LoadError {
                    path: config_path.display().to_string(),
                    source: format!("Failed to load {}: {}", filename, e),
                })?;
            merge_fn(config, cfg)
                .map_err(|e| ConfigError::MergeError(format!("Failed to merge {}: {}", filename, e)))?;
            debug!("Loaded {} for survey", filename);
        }
        Ok(())
    }
    
    /// Load override configurations with precedence
    fn load_override_configs(&self, config: &mut SurveyConfig, survey_code: &str, environment: &str) -> Result<()> {
        let survey_dir = self.config_dir.join(path::survey_dir(survey_code));
        let overrides_dir = survey_dir.join("overrides");
        
        // Load defaults.yml
        let defaults_path = overrides_dir.join("defaults.yml");
        if defaults_path.exists() {
            let override_config = file::read_yaml::<OverrideConfig, _>(&defaults_path)
                .map_err(|e| ConfigError::LoadError {
                    path: defaults_path.display().to_string(),
                    source: format!("Failed to load defaults.yml: {}", e),
                })?;
            config.merge_override(&override_config, "defaults")
                .map_err(|e| ConfigError::MergeError(format!("Failed to merge defaults: {}", e)))?;
            debug!("Loaded defaults.yml for {}", survey_code);
        }
        
        // Load environment-specific overrides
        let env_path = overrides_dir.join("env").join(format!("{}.yml", environment));
        if env_path.exists() {
            let override_config = file::read_yaml::<OverrideConfig, _>(&env_path)
                .map_err(|e| ConfigError::LoadError {
                    path: env_path.display().to_string(),
                    source: format!("Failed to load {}.yml: {}", environment, e),
                })?;
            config.merge_override(&override_config, &format!("env/{}", environment))
                .map_err(|e| ConfigError::MergeError(format!("Failed to merge env overrides: {}", e)))?;
            debug!("Loaded {}.yml for {}", environment, survey_code);
        }
        
        // Load local.yml (highest precedence)
        let local_path = overrides_dir.join("local.yml");
        if local_path.exists() {
            let override_config = file::read_yaml::<OverrideConfig, _>(&local_path)
                .map_err(|e| ConfigError::LoadError {
                    path: local_path.display().to_string(),
                    source: format!("Failed to load local.yml: {}", e),
                })?;
            config.merge_override(&override_config, "local")
                .map_err(|e| ConfigError::MergeError(format!("Failed to merge local overrides: {}", e)))?;
            debug!("Loaded local.yml for {}", survey_code);
        }
        
        Ok(())
    }

    /// Load configuration from multiple sources with precedence
    pub fn load_from_sources(&self, sources: Vec<ConfigSource>) -> Result<Config> {
        let mut config = None;

        for source in sources {
            match source {
                ConfigSource::File(path) => {
                    if path.exists() {
                        config = Some(self.load_from_file(&path)?);
                        break;
                    }
                }
                ConfigSource::String(content, format) => {
                    config = Some(self.load_from_string(&content, format)?);
                    break;
                }
                ConfigSource::Environment => {
                    // Environment source requires a survey code, skip for now
                    continue;
                }
                ConfigSource::Default(survey_code) => {
                    config = Some(Config::new(&survey_code));
                }
            }
        }

        config.ok_or_else(|| crate::error::types::Error::Config(ConfigError::LoadError {
            path: "multiple sources".to_string(),
            source: "No valid configuration source found".to_string(),
        }))
    }

    /// Detect configuration format from file extension
    fn detect_format(&self, path: &Path) -> Result<ConfigFormat> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());

        match extension.as_deref() {
            Some("yml") | Some("yaml") => Ok(ConfigFormat::Yaml),
            Some("json") => Ok(ConfigFormat::Json),
            Some(ext) => Err(ConfigError::UnsupportedFormat(ext.to_string())),
            None => Err(ConfigError::UnsupportedFormat("No file extension".to_string())),
        }
    }

    /// Parse configuration content based on format
    fn parse_config(&self, content: &str, format: ConfigFormat) -> Result<Config> {
        match format {
            ConfigFormat::Yaml => {
                serde_yaml::from_str(content)
                    .map_err(|e| ConfigError::ParseError {
                        message: format!("YAML parse error: {}", e),
                        line: None,
                        column: None,
                    }.into())
            }
            ConfigFormat::Json => {
                serde_json::from_str(content)
                    .map_err(|e| ConfigError::ParseError {
                        message: format!("JSON parse error: {}", e),
                        line: None,
                        column: None,
                    }.into())
            }
            ConfigFormat::Auto => {
                // Try YAML first, then JSON
                if let Ok(config) = serde_yaml::from_str::<Config>(content) {
                    Ok(config)
                } else {
                    serde_json::from_str(content)
                        .map_err(|e| ConfigError::ParseError {
                            message: format!("Auto-detection failed, JSON parse error: {}", e),
                            line: None,
                            column: None,
                        }.into())
                }
            }
        }
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, config: &Config, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Validate the path
        let path_utils = path::PathUtils::new();
        path_utils.validate_path(path)
            .map_err(|e| ConfigError::LoadError {
                path: "unknown".to_string(),
                source: format!("Invalid path: {}", e),
            })?;

        // Resolve relative paths
        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config_dir.join(path)
        };

        // Determine format from file extension
        let format = self.detect_format(&full_path)?;

        // Serialize configuration
        let content = match format {
            ConfigFormat::Yaml => {
                serde_yaml::to_string(config)
                    .map_err(|e| ConfigError::SerializationError(format!("YAML serialization error: {}", e)))?
            }
            ConfigFormat::Json => {
                serde_json::to_string_pretty(config)
                    .map_err(|e| ConfigError::SerializationError(format!("JSON serialization error: {}", e)))?
            }
            ConfigFormat::Auto => {
                // Default to YAML for auto format
                serde_yaml::to_string(config)
                    .map_err(|e| ConfigError::SerializationError(format!("YAML serialization error: {}", e)))?
            }
        };

        // Write to file
        file::write_string(&full_path, &content)
            .map_err(|e| ConfigError::MergeError(format!("Failed to write file: {}", e)))?;

        Ok(())
    }

    /// List available survey configurations
    pub fn list_survey_configs(&self) -> Result<Vec<String>> {
        let surveys_dir = self.config_dir.join("surveys");
        
        if !surveys_dir.exists() {
            return Ok(Vec::new());
        }

        let file_utils = file::FileUtils::new();
        let mut survey_codes = Vec::new();

        // Look for YAML files
        if let Ok(yaml_files) = file_utils.list_files(&surveys_dir, Some("yml")) {
            for file_path in yaml_files {
                if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                    survey_codes.push(stem.to_uppercase());
                }
            }
        }

        // Look for JSON files
        if let Ok(json_files) = file_utils.list_files(&surveys_dir, Some("json")) {
            for file_path in json_files {
                if let Some(stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                    let code = stem.to_uppercase();
                    if !survey_codes.contains(&code) {
                        survey_codes.push(code);
                    }
                }
            }
        }

        survey_codes.sort();
        Ok(survey_codes)
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_config_loader_new() {
        let loader = ConfigLoader::new();
        assert!(loader.config_dir.to_string_lossy().contains("config"));
    }

    #[test]
    fn test_detect_format() {
        let loader = ConfigLoader::new();
        
        assert_eq!(loader.detect_format(Path::new("test.yml")).unwrap(), ConfigFormat::Yaml);
        assert_eq!(loader.detect_format(Path::new("test.yaml")).unwrap(), ConfigFormat::Yaml);
        assert_eq!(loader.detect_format(Path::new("test.json")).unwrap(), ConfigFormat::Json);
        
        assert!(loader.detect_format(Path::new("test.txt")).is_err());
        assert!(loader.detect_format(Path::new("test")).is_err());
    }

    #[test]
    fn test_parse_config_yaml() {
        let loader = ConfigLoader::new();
        let yaml_content = r#"
survey:
  code: "AP"
  name: "Average Price Data"
processing:
  strategy: "in_memory"
  max_threads: 4
files: {}
output:
  formats: ["csv"]
error_handling:
  on_error: "continue"
"#;

        let config = loader.parse_config(yaml_content, ConfigFormat::Yaml).unwrap();
        assert_eq!(config.survey.code, "AP");
        assert_eq!(config.survey.name, "Average Price Data");
        assert_eq!(config.processing.max_threads, 4);
    }

    #[test]
    fn test_parse_config_json() {
        let loader = ConfigLoader::new();
        let json_content = r#"
{
  "survey": {
    "code": "BD",
    "name": "Business Dynamics"
  },
  "processing": {
    "strategy": "chunked",
    "max_threads": 8
  },
  "files": {},
  "output": {
    "formats": ["parquet"]
  },
  "error_handling": {
    "on_error": "abort"
  }
}
"#;

        let config = loader.parse_config(json_content, ConfigFormat::Json).unwrap();
        assert_eq!(config.survey.code, "BD");
        assert_eq!(config.survey.name, "Business Dynamics");
        assert_eq!(config.processing.max_threads, 8);
    }

    #[test]
    fn test_load_from_string() {
        let loader = ConfigLoader::new();
        let yaml_content = r#"
survey:
  code: "CE"
  name: "Current Employment Statistics"
processing:
  strategy: "mmap"
files: {}
output:
  formats: ["json"]
error_handling:
  on_error: "continue"
"#;

        let config = loader.load_from_string(yaml_content, ConfigFormat::Yaml).unwrap();
        assert_eq!(config.survey.code, "CE");
        assert_eq!(config.processing.strategy, crate::config::model::ProcessingStrategy::Mmap);
    }

    #[test]
    fn test_load_from_environment() {
        let loader = ConfigLoader::new();
        
        // Set environment variables
        std::env::set_var("RUSTY_PROCESSING_STRATEGY", "chunked");
        std::env::set_var("RUSTY_MAX_THREADS", "16");
        std::env::set_var("RUSTY_OUTPUT_DIR", "custom/output");
        
        let config = loader.load_from_environment("TEST").unwrap();
        assert_eq!(config.survey.code, "TEST");
        assert_eq!(config.processing.strategy, crate::config::model::ProcessingStrategy::Chunked);
        assert_eq!(config.processing.max_threads, 16);
        assert_eq!(config.output.output_dir, "custom/output");
        
        // Clean up environment variables
        std::env::remove_var("RUSTY_PROCESSING_STRATEGY");
        std::env::remove_var("RUSTY_MAX_THREADS");
        std::env::remove_var("RUSTY_OUTPUT_DIR");
    }

    #[test]
    fn test_save_and_load_file() {
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_config_dir(temp_dir.path());
        
        // Create a test configuration
        let mut config = Config::new("TEST");
        config.processing.max_threads = 12;
        config.output.output_dir = "test/output".to_string();
        
        // Save to file
        let config_path = temp_dir.path().join("test.yml");
        loader.save_to_file(&config, &config_path).unwrap();
        
        // Load from file
        let loaded_config = loader.load_from_file(&config_path).unwrap();
        assert_eq!(loaded_config.survey.code, "TEST");
        assert_eq!(loaded_config.processing.max_threads, 12);
        assert_eq!(loaded_config.output.output_dir, "test/output");
    }

    #[test]
    fn test_load_for_survey_default() {
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_config_dir(temp_dir.path());
        
        // Load for non-existent survey (should create default)
        let config = loader.load_for_survey("XX").unwrap();
        assert_eq!(config.survey.code, "XX");
        assert_eq!(config.survey.name, "XX Survey");
    }

    #[test]
    fn test_list_survey_configs() {
        let temp_dir = TempDir::new().unwrap();
        let surveys_dir = temp_dir.path().join("surveys");
        fs::create_dir_all(&surveys_dir).unwrap();
        
        // Create test config files
        fs::write(surveys_dir.join("ap.yml"), "survey:\n  code: AP\nprocessing: {}\nfiles: {}\noutput: {}\nerror_handling: {}").unwrap();
        fs::write(surveys_dir.join("bd.json"), r#"{"survey":{"code":"BD"},"processing":{},"files":{},"output":{},"error_handling":{}}"#).unwrap();
        
        let loader = ConfigLoader::with_config_dir(temp_dir.path());
        let survey_codes = loader.list_survey_configs().unwrap();
        
        assert_eq!(survey_codes.len(), 2);
        assert!(survey_codes.contains(&"AP".to_string()));
        assert!(survey_codes.contains(&"BD".to_string()));
    }
}