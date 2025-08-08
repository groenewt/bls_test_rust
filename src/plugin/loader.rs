//! Plugin loader implementation for dynamic plugin loading and management.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::error::types::{Error, Result};
use crate::plugin::traits::{Plugin, PluginConfig, PluginMetadata};

/// Plugin loader trait for dynamic plugin loading.
#[async_trait]
pub trait PluginLoader: Send + Sync {
    /// Loads a plugin from the specified path.
    async fn load_plugin(&mut self, path: &Path, config: PluginConfig) -> Result<Box<dyn Plugin>>;

    /// Unloads a plugin by its ID.
    async fn unload_plugin(&mut self, plugin_id: &str) -> Result<()>;

    /// Lists all loaded plugins.
    fn list_loaded_plugins(&self) -> Vec<String>;

    /// Checks if a plugin is loaded.
    fn is_plugin_loaded(&self, plugin_id: &str) -> bool;

    /// Validates a plugin before loading.
    async fn validate_plugin(&self, path: &Path) -> Result<PluginMetadata>;

    /// Returns loader statistics.
    fn loader_stats(&self) -> LoaderStats;
}

/// Default plugin loader implementation.
pub struct DefaultPluginLoader {
    loaded_plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    config: LoaderConfig,
    stats: LoaderStats,
}

/// Information about a loaded plugin.
#[derive(Clone)]
struct LoadedPlugin {
    plugin: Arc<tokio::sync::RwLock<Box<dyn Plugin>>>,
    metadata: PluginMetadata,
    loaded_at: SystemTime,
    path: PathBuf,
}

impl std::fmt::Debug for LoadedPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedPlugin")
            .field("metadata", &self.metadata)
            .field("loaded_at", &self.loaded_at)
            .field("path", &self.path)
            .field("plugin", &"<Plugin instance>")
            .finish()
    }
}

/// Plugin loader configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderConfig {
    /// Maximum number of plugins that can be loaded simultaneously.
    pub max_plugins: usize,
    /// Whether to enable plugin signature verification.
    pub verify_signatures: bool,
    /// Allowed plugin directories.
    pub allowed_directories: Vec<PathBuf>,
    /// Plugin loading timeout in seconds.
    pub loading_timeout_seconds: u64,
    /// Whether to enable plugin sandboxing.
    pub enable_sandboxing: bool,
}

/// Plugin loader statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderStats {
    /// Number of plugins successfully loaded.
    pub plugins_loaded: u64,
    /// Number of plugin loading failures.
    pub loading_failures: u64,
    /// Number of plugins currently loaded.
    pub currently_loaded: u64,
    /// Total loading time in milliseconds.
    pub total_loading_time_ms: u64,
    /// Average loading time per plugin in milliseconds.
    pub avg_loading_time_ms: f64,
}

impl DefaultPluginLoader {
    /// Creates a new plugin loader with the specified configuration.
    pub fn new(config: LoaderConfig) -> Self {
        Self {
            loaded_plugins: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: LoaderStats::new(),
        }
    }

    /// Creates a plugin loader with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(LoaderConfig::default())
    }

    /// Validates that the plugin path is allowed.
    fn validate_plugin_path(&self, path: &Path) -> Result<()> {
        if self.config.allowed_directories.is_empty() {
            return Ok(());
        }

        let canonical_path = path.canonicalize().map_err(|e| {
            Error::Plugin(crate::error::types::PluginError::LoadError {
                plugin: path.to_string_lossy().to_string(),
                source: format!("Failed to canonicalize plugin path: {e}"),
            })
        })?;

        for allowed_dir in &self.config.allowed_directories {
            if canonical_path.starts_with(allowed_dir) {
                return Ok(());
            }
        }

        Err(Error::Plugin(
            crate::error::types::PluginError::ConfigurationError {
                plugin: path.to_string_lossy().to_string(),
                message: format!("Plugin path {path:?} is not in allowed directories"),
            },
        ))
    }

    /// Checks if the maximum number of plugins is reached.
    fn check_plugin_limit(&self) -> Result<()> {
        let loaded_plugins = self.loaded_plugins.read().map_err(|_| {
            Error::Plugin(crate::error::types::PluginError::ExecutionError {
                plugin: "plugin_loader".to_string(),
                message: "Failed to acquire read lock".to_string(),
            })
        })?;
        let loaded_count = loaded_plugins.len();

        if loaded_count >= self.config.max_plugins {
            return Err(Error::Plugin(
                crate::error::types::PluginError::ConfigurationError {
                    plugin: "plugin_loader".to_string(),
                    message: format!(
                        "Maximum number of plugins ({}) reached",
                        self.config.max_plugins
                    ),
                },
            ));
        }

        Ok(())
    }

    /// Verifies plugin signature if enabled.
    async fn verify_plugin_signature(&self, _path: &Path) -> Result<()> {
        if !self.config.verify_signatures {
            return Ok(());
        }

        // TODO: Implement actual signature verification
        // For now, just return Ok() as a placeholder
        Ok(())
    }

    /// Loads plugin metadata from the plugin file.
    async fn load_plugin_metadata(&self, path: &Path) -> Result<PluginMetadata> {
        // TODO: Implement actual metadata loading from plugin file
        // For now, create a placeholder metadata
        Ok(PluginMetadata {
            id: format!(
                "plugin_{}",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ),
            name: format!(
                "Plugin {}",
                path.file_stem().unwrap_or_default().to_string_lossy()
            ),
            version: "1.0.0".to_string(),
            description: "Dynamically loaded plugin".to_string(),
            author: "Unknown".to_string(),
            supported_surveys: vec![],
            created_at: SystemTime::now(),
        })
    }

    /// Creates a plugin instance from the loaded library.
    async fn create_plugin_instance(
        &self,
        _path: &Path,
        _metadata: &PluginMetadata,
    ) -> Result<Box<dyn Plugin>> {
        // TODO: Implement actual plugin instantiation from dynamic library
        // For now, return an error as this requires unsafe code and dynamic loading
        Err(Error::Plugin(
            crate::error::types::PluginError::ExecutionError {
                plugin: "unknown".to_string(),
                message: "Plugin instantiation not yet implemented".to_string(),
            },
        ))
    }
}

#[async_trait]
impl PluginLoader for DefaultPluginLoader {
    async fn load_plugin(&mut self, path: &Path, config: PluginConfig) -> Result<Box<dyn Plugin>> {
        let start_time = SystemTime::now();

        // Validate plugin path
        self.validate_plugin_path(path)?;

        // Check plugin limit
        self.check_plugin_limit()?;

        // Verify plugin signature
        self.verify_plugin_signature(path).await?;

        // Load plugin metadata
        let metadata = self.load_plugin_metadata(path).await?;

        // Check if plugin is already loaded
        {
            let loaded_plugins = self.loaded_plugins.read().map_err(|_| {
                Error::Plugin(crate::error::types::PluginError::ExecutionError {
                    plugin: "plugin_loader".to_string(),
                    message: "Failed to acquire read lock".to_string(),
                })
            })?;

            if loaded_plugins.contains_key(&metadata.id) {
                return Err(Error::Plugin(crate::error::types::PluginError::LoadError {
                    plugin: metadata.id.clone(),
                    source: "Plugin is already loaded".to_string(),
                }));
            }
        }

        // Create plugin instance
        let mut plugin = self.create_plugin_instance(path, &metadata).await?;

        // Initialize plugin
        plugin.initialize(config).await?;

        // Store loaded plugin information
        let loaded_plugin = LoadedPlugin {
            plugin: Arc::new(tokio::sync::RwLock::new(plugin)),
            metadata: metadata.clone(),
            loaded_at: SystemTime::now(),
            path: path.to_path_buf(),
        };

        {
            let mut loaded_plugins = self.loaded_plugins.write().map_err(|_| {
                Error::Plugin(crate::error::types::PluginError::ExecutionError {
                    plugin: "plugin_loader".to_string(),
                    message: "Failed to acquire write lock".to_string(),
                })
            })?;
            loaded_plugins.insert(metadata.id.clone(), loaded_plugin);
        }

        // Update statistics
        self.stats.plugins_loaded += 1;
        self.stats.currently_loaded += 1;

        if let Ok(elapsed) = start_time.elapsed() {
            let loading_time_ms = elapsed.as_millis() as u64;
            self.stats.total_loading_time_ms += loading_time_ms;
            self.stats.avg_loading_time_ms =
                self.stats.total_loading_time_ms as f64 / self.stats.plugins_loaded as f64;
        }

        // Return a clone of the plugin (this is a placeholder - actual implementation would differ)
        Err(Error::Plugin(
            crate::error::types::PluginError::ExecutionError {
                plugin: "plugin_loader".to_string(),
                message: "Plugin loading not fully implemented".to_string(),
            },
        ))
    }

    async fn unload_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let loaded_plugin = {
            let mut loaded_plugins = self.loaded_plugins.write().map_err(|_| {
                Error::Plugin(crate::error::types::PluginError::ExecutionError {
                    plugin: "plugin_loader".to_string(),
                    message: "Failed to acquire write lock".to_string(),
                })
            })?;

            loaded_plugins.remove(plugin_id).ok_or_else(|| {
                Error::Plugin(crate::error::types::PluginError::NotFoundError {
                    plugin: plugin_id.to_string(),
                    message: format!("Plugin '{plugin_id}' not found"),
                })
            })?
        };

        // Shutdown the plugin
        let plugin_arc = loaded_plugin.plugin.clone();
        {
            let mut plugin = plugin_arc.write().await;
            plugin.shutdown().await?;
        }

        // Update statistics
        self.stats.currently_loaded = self.stats.currently_loaded.saturating_sub(1);

        Ok(())
    }

    fn list_loaded_plugins(&self) -> Vec<String> {
        self.loaded_plugins
            .read()
            .map(|plugins| plugins.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn is_plugin_loaded(&self, plugin_id: &str) -> bool {
        self.loaded_plugins
            .read()
            .map(|plugins| plugins.contains_key(plugin_id))
            .unwrap_or(false)
    }

    async fn validate_plugin(&self, path: &Path) -> Result<PluginMetadata> {
        // Validate plugin path
        self.validate_plugin_path(path)?;

        // Verify plugin signature
        self.verify_plugin_signature(path).await?;

        // Load and return metadata
        self.load_plugin_metadata(path).await
    }

    fn loader_stats(&self) -> LoaderStats {
        self.stats.clone()
    }
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            max_plugins: 50,
            verify_signatures: true,
            allowed_directories: vec![],
            loading_timeout_seconds: 30,
            enable_sandboxing: true,
        }
    }
}

impl Default for LoaderStats {
    fn default() -> Self {
        Self::new()
    }
}

impl LoaderStats {
    /// Creates a new LoaderStats instance with default values.
    pub fn new() -> Self {
        Self {
            plugins_loaded: 0,
            loading_failures: 0,
            currently_loaded: 0,
            total_loading_time_ms: 0,
            avg_loading_time_ms: 0.0,
        }
    }

    /// Records a loading failure.
    pub fn record_failure(&mut self) {
        self.loading_failures += 1;
    }

    /// Returns the success rate as a percentage.
    pub fn success_rate(&self) -> f64 {
        let total_attempts = self.plugins_loaded + self.loading_failures;
        if total_attempts == 0 {
            0.0
        } else {
            (self.plugins_loaded as f64 / total_attempts as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert_eq!(config.max_plugins, 50);
        assert!(config.verify_signatures);
        assert!(config.enable_sandboxing);
        assert_eq!(config.loading_timeout_seconds, 30);
    }

    #[test]
    fn test_loader_stats_new() {
        let stats = LoaderStats::new();
        assert_eq!(stats.plugins_loaded, 0);
        assert_eq!(stats.loading_failures, 0);
        assert_eq!(stats.success_rate(), 0.0);
    }

    #[test]
    fn test_loader_stats_success_rate() {
        let mut stats = LoaderStats::new();
        stats.plugins_loaded = 8;
        stats.loading_failures = 2;
        assert_eq!(stats.success_rate(), 80.0);
    }

    #[tokio::test]
    async fn test_default_plugin_loader_creation() {
        let loader = DefaultPluginLoader::with_defaults();
        assert_eq!(loader.list_loaded_plugins().len(), 0);
        assert_eq!(loader.loader_stats().currently_loaded, 0);
    }

    #[tokio::test]
    async fn test_plugin_path_validation() {
        let mut config = LoaderConfig::default();
        config.allowed_directories = vec![PathBuf::from("/allowed/path")];

        let loader = DefaultPluginLoader::new(config);

        // This should fail because the path is not in allowed directories
        let result = loader.validate_plugin_path(&PathBuf::from("/forbidden/path"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_plugin_limit_check() {
        let mut config = LoaderConfig::default();
        config.max_plugins = 0; // Set limit to 0 for testing

        let loader = DefaultPluginLoader::new(config);
        let result = loader.check_plugin_limit();
        assert!(result.is_err());
    }
}
