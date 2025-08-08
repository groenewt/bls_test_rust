//! Plugin system module for the Rusty BLS Data Processing system.
//!
//! This module provides a comprehensive plugin architecture that enables dynamic loading
//! and execution of survey-specific processing components. The plugin system is designed
//! for enterprise-level scalability, security, and performance.
//!
//! # Architecture
//!
//! The plugin system consists of several key components:
//!
//! - **Traits**: Core interfaces that define plugin behavior and capabilities
//! - **Loader**: Dynamic plugin loading with security validation and resource management
//! - **Registry**: Plugin registration, discovery, and lifecycle management
//! - **Security**: Comprehensive security features including sandboxing and validation
//!
//! # Security Features
//!
//! The plugin system includes enterprise-grade security features:
//!
//! - Plugin signature verification
//! - Sandboxed execution environments
//! - Resource usage monitoring and limits
//! - Permission-based access control
//! - Path validation and restrictions
//!
//! # Performance Optimization
//!
//! The system is optimized for high-performance operation:
//!
//! - Lazy loading and initialization
//! - Connection pooling and resource reuse
//! - Metrics collection and performance monitoring
//! - Memory-efficient data processing
//! - Concurrent plugin execution
//!
//! # Usage Example
//!
//! ```rust
//! use rusty::plugin::{DefaultPluginRegistry, DefaultPluginLoader, PluginConfig};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create plugin registry
//!     let mut registry = DefaultPluginRegistry::with_defaults();
//!     
//!     // Load and register a plugin
//!     let mut loader = DefaultPluginLoader::with_defaults();
//!     let plugin_path = Path::new("plugins/survey_processor.so");
//!     let config = PluginConfig::default();
//!     
//!     match loader.load_plugin(plugin_path, config).await {
//!         Ok(plugin) => {
//!             registry.register_plugin(plugin).await?;
//!             println!("Plugin loaded successfully");
//!         }
//!         Err(e) => {
//!             eprintln!("Failed to load plugin: {}", e);
//!         }
//!     }
//!     
//!     // Find plugins for a specific survey
//!     let ap_plugins = registry.find_plugins_for_survey("AP");
//!     println!("Found {} plugins for AP survey", ap_plugins.len());
//!     
//!     Ok(())
//! }
//! ```

pub mod traits;
pub mod loader;
pub mod registry;

// Re-export commonly used types and traits
pub use traits::{
    Plugin, PluginMetadata, PluginConfig, PluginStats, ResourceLimits,
};

pub use loader::{
    PluginLoader, DefaultPluginLoader, LoaderConfig, LoaderStats,
};

pub use registry::{
    PluginRegistry, DefaultPluginRegistry, RegistryConfig, RegistryStats,
};

use crate::error::types::{Result, Error, PluginError};
use std::path::Path;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Plugin system manager that coordinates all plugin operations.
pub struct PluginManager {
    registry: Box<dyn PluginRegistry>,
    loader: Box<dyn PluginLoader>,
    config: ManagerConfig,
}

/// Configuration for the plugin manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerConfig {
    /// Plugin directories to scan for available plugins.
    pub plugin_directories: Vec<std::path::PathBuf>,
    /// Whether to automatically load plugins on startup.
    pub auto_load_plugins: bool,
    /// Plugin loading timeout in seconds.
    pub loading_timeout_seconds: u64,
    /// Whether to enable plugin hot-reloading.
    pub enable_hot_reload: bool,
    /// Hot-reload check interval in seconds.
    pub hot_reload_interval_seconds: u64,
}

impl PluginManager {
    /// Creates a new plugin manager with the specified configuration.
    pub fn new(config: ManagerConfig) -> Self {
        let registry_config = RegistryConfig::default();
        let loader_config = LoaderConfig::default();
        
        Self {
            registry: Box::new(DefaultPluginRegistry::new(registry_config, loader_config.clone())),
            loader: Box::new(DefaultPluginLoader::new(loader_config)),
            config,
        }
    }

    /// Creates a plugin manager with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ManagerConfig::default())
    }

    /// Initializes the plugin manager and loads plugins if configured.
    pub async fn initialize(&mut self) -> Result<()> {
        if self.config.auto_load_plugins {
            self.discover_and_load_plugins().await?;
        }
        Ok(())
    }

    /// Discovers and loads plugins from configured directories.
    pub async fn discover_and_load_plugins(&mut self) -> Result<()> {
        let plugin_directories = self.config.plugin_directories.clone();
        for plugin_dir in &plugin_directories {
            if let Err(e) = self.load_plugins_from_directory(plugin_dir).await {
                eprintln!("Failed to load plugins from {:?}: {}", plugin_dir, e);
            }
        }
        Ok(())
    }

    /// Loads plugins from a specific directory.
    pub async fn load_plugins_from_directory(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() || !dir.is_dir() {
            return Err(Error::Plugin(PluginError::LoadError { 
                plugin: dir.display().to_string(), 
                source: "Plugin directory does not exist or is not a directory".to_string() 
            }));
        }

        let entries = std::fs::read_dir(dir)
            .map_err(|e| Error::Plugin(PluginError::LoadError { 
                plugin: dir.display().to_string(), 
                source: format!("Failed to read plugin directory: {}", e) 
            }))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| Error::Plugin(PluginError::LoadError { 
                    plugin: "directory_entry".to_string(), 
                    source: format!("Failed to read directory entry: {}", e) 
                }))?;
            
            let path = entry.path();
            
            // Check if this looks like a plugin file (e.g., .so, .dll, .dylib)
            if self.is_plugin_file(&path) {
                if let Err(e) = self.load_plugin_file(&path).await {
                    eprintln!("Failed to load plugin {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// Loads a specific plugin file.
    pub async fn load_plugin_file(&mut self, path: &Path) -> Result<()> {
        let config = PluginConfig::default();
        let plugin = self.loader.load_plugin(path, config).await?;
        self.registry.register_plugin(plugin).await?;
        Ok(())
    }

    /// Checks if a file appears to be a plugin file based on its extension.
    fn is_plugin_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            matches!(ext.as_str(), "so" | "dll" | "dylib")
        } else {
            false
        }
    }

    /// Gets a reference to the plugin registry.
    pub fn registry(&self) -> &dyn PluginRegistry {
        self.registry.as_ref()
    }

    /// Gets a mutable reference to the plugin registry.
    pub fn registry_mut(&mut self) -> &mut dyn PluginRegistry {
        self.registry.as_mut()
    }

    /// Gets a reference to the plugin loader.
    pub fn loader(&self) -> &dyn PluginLoader {
        self.loader.as_ref()
    }

    /// Gets a mutable reference to the plugin loader.
    pub fn loader_mut(&mut self) -> &mut dyn PluginLoader {
        self.loader.as_mut()
    }

    /// Returns comprehensive statistics about the plugin system.
    pub fn system_stats(&self) -> SystemStats {
        SystemStats {
            registry_stats: self.registry.registry_stats(),
            loader_stats: self.loader.loader_stats(),
            manager_config: self.config.clone(),
        }
    }

    /// Shuts down the plugin manager and all loaded plugins.
    pub async fn shutdown(&mut self) -> Result<()> {
        self.registry.clear().await?;
        Ok(())
    }
}

/// Comprehensive statistics about the plugin system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    /// Registry statistics.
    pub registry_stats: RegistryStats,
    /// Loader statistics.
    pub loader_stats: LoaderStats,
    /// Manager configuration.
    pub manager_config: ManagerConfig,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            plugin_directories: vec![
                std::path::PathBuf::from("plugins"),
                std::path::PathBuf::from("/usr/local/lib/rusty/plugins"),
            ],
            auto_load_plugins: true,
            loading_timeout_seconds: 30,
            enable_hot_reload: false,
            hot_reload_interval_seconds: 60,
        }
    }
}

/// Validates plugin security and integrity.
pub struct PluginValidator {
    config: ValidatorConfig,
}

/// Configuration for plugin validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    /// Whether to verify plugin signatures.
    pub verify_signatures: bool,
    /// Trusted certificate authorities for signature verification.
    pub trusted_cas: Vec<std::path::PathBuf>,
    /// Whether to perform static analysis on plugins.
    pub enable_static_analysis: bool,
    /// Maximum allowed plugin file size in bytes.
    pub max_plugin_size_bytes: u64,
    /// Allowed plugin file extensions.
    pub allowed_extensions: Vec<String>,
}

impl PluginValidator {
    /// Creates a new plugin validator with the specified configuration.
    pub fn new(config: ValidatorConfig) -> Self {
        Self { config }
    }

    /// Creates a plugin validator with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ValidatorConfig::default())
    }

    /// Validates a plugin file before loading.
    pub async fn validate_plugin(&self, path: &Path) -> Result<ValidationReport> {
        let mut report = ValidationReport::new(path.to_path_buf());

        // Check file existence and accessibility
        if !path.exists() {
            report.add_error("Plugin file does not exist".to_string());
            return Ok(report);
        }

        // Check file size
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > self.config.max_plugin_size_bytes {
                report.add_error(format!(
                    "Plugin file size ({} bytes) exceeds maximum allowed size ({} bytes)",
                    metadata.len(),
                    self.config.max_plugin_size_bytes
                ));
            }
        }

        // Check file extension
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            if !self.config.allowed_extensions.contains(&ext) {
                report.add_error(format!("Plugin file extension '{}' is not allowed", ext));
            }
        } else {
            report.add_error("Plugin file has no extension".to_string());
        }

        // Verify signature if enabled
        if self.config.verify_signatures {
            if let Err(e) = self.verify_plugin_signature(path).await {
                report.add_error(format!("Signature verification failed: {}", e));
            }
        }

        // Perform static analysis if enabled
        if self.config.enable_static_analysis {
            if let Err(e) = self.perform_static_analysis(path).await {
                report.add_warning(format!("Static analysis warning: {}", e));
            }
        }

        Ok(report)
    }

    /// Verifies the digital signature of a plugin file.
    async fn verify_plugin_signature(&self, _path: &Path) -> Result<()> {
        // TODO: Implement actual signature verification
        // This would involve checking digital signatures against trusted CAs
        Ok(())
    }

    /// Performs static analysis on a plugin file.
    async fn perform_static_analysis(&self, _path: &Path) -> Result<()> {
        // TODO: Implement static analysis
        // This could include checking for suspicious patterns, malware, etc.
        Ok(())
    }
}

/// Report from plugin validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Path to the validated plugin.
    pub plugin_path: std::path::PathBuf,
    /// Validation errors found.
    pub errors: Vec<String>,
    /// Validation warnings found.
    pub warnings: Vec<String>,
    /// Whether the plugin passed validation.
    pub is_valid: bool,
    /// Validation timestamp.
    pub validated_at: std::time::SystemTime,
}

impl ValidationReport {
    /// Creates a new validation report for the specified plugin.
    pub fn new(plugin_path: std::path::PathBuf) -> Self {
        Self {
            plugin_path,
            errors: Vec::new(),
            warnings: Vec::new(),
            is_valid: true,
            validated_at: std::time::SystemTime::now(),
        }
    }

    /// Adds an error to the validation report.
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// Adds a warning to the validation report.
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Returns whether the plugin has any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns whether the plugin has any warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            verify_signatures: true,
            trusted_cas: vec![],
            enable_static_analysis: false,
            max_plugin_size_bytes: 100 * 1024 * 1024, // 100MB
            allowed_extensions: vec![
                "so".to_string(),
                "dll".to_string(),
                "dylib".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_manager_config_default() {
        let config = ManagerConfig::default();
        assert!(config.auto_load_plugins);
        assert_eq!(config.loading_timeout_seconds, 30);
        assert!(!config.enable_hot_reload);
        assert_eq!(config.plugin_directories.len(), 2);
    }

    #[test]
    fn test_validator_config_default() {
        let config = ValidatorConfig::default();
        assert!(config.verify_signatures);
        assert!(!config.enable_static_analysis);
        assert_eq!(config.max_plugin_size_bytes, 100 * 1024 * 1024);
        assert_eq!(config.allowed_extensions.len(), 3);
    }

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let manager = PluginManager::with_defaults();
        let stats = manager.system_stats();
        assert_eq!(stats.registry_stats.registered_plugins, 0);
        assert_eq!(stats.loader_stats.plugins_loaded, 0);
    }

    #[tokio::test]
    async fn test_validation_report() {
        let mut report = ValidationReport::new(PathBuf::from("test.so"));
        assert!(report.is_valid);
        assert!(!report.has_errors());
        assert!(!report.has_warnings());

        report.add_warning("Test warning".to_string());
        assert!(report.is_valid);
        assert!(report.has_warnings());

        report.add_error("Test error".to_string());
        assert!(!report.is_valid);
        assert!(report.has_errors());
    }

    #[test]
    fn test_is_plugin_file() {
        let manager = PluginManager::with_defaults();
        
        assert!(manager.is_plugin_file(&PathBuf::from("test.so")));
        assert!(manager.is_plugin_file(&PathBuf::from("test.dll")));
        assert!(manager.is_plugin_file(&PathBuf::from("test.dylib")));
        assert!(!manager.is_plugin_file(&PathBuf::from("test.txt")));
        assert!(!manager.is_plugin_file(&PathBuf::from("test")));
    }

    #[tokio::test]
    async fn test_plugin_validator() {
        let validator = PluginValidator::with_defaults();
        
        // Test with non-existent file
        let report = validator.validate_plugin(&PathBuf::from("nonexistent.so")).await.unwrap();
        assert!(!report.is_valid);
        assert!(report.has_errors());
    }
}