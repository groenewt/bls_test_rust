//! Plugin registry implementation for managing and discovering plugins.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::plugin::traits::{Plugin, PluginMetadata, PluginConfig};
use crate::plugin::loader::{PluginLoader, DefaultPluginLoader, LoaderConfig};
use crate::error::types::{Result, Error, PluginError};
use crate::error::recovery::HealthStatus;

/// Plugin registry trait for managing plugins.
#[async_trait]
pub trait PluginRegistry: Send + Sync {
    /// Registers a plugin with the registry.
    async fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<()>;

    /// Unregisters a plugin from the registry.
    async fn unregister_plugin(&mut self, plugin_id: &str) -> Result<()>;

    /// Gets a plugin by its ID.
    fn get_plugin(&self, plugin_id: &str) -> Result<Arc<tokio::sync::RwLock<Box<dyn Plugin>>>>;

    /// Lists all registered plugins.
    fn list_plugins(&self) -> Vec<String>;

    /// Finds plugins that support a specific survey.
    fn find_plugins_for_survey(&self, survey_code: &str) -> Vec<String>;

    /// Gets plugin metadata by ID.
    fn get_plugin_metadata(&self, plugin_id: &str) -> Result<PluginMetadata>;

    /// Checks if a plugin is registered.
    fn is_plugin_registered(&self, plugin_id: &str) -> bool;

    /// Returns registry statistics.
    fn registry_stats(&self) -> RegistryStats;

    /// Clears all registered plugins.
    async fn clear(&mut self) -> Result<()>;
}

/// Default plugin registry implementation.
pub struct DefaultPluginRegistry {
    plugins: Arc<RwLock<HashMap<String, RegisteredPlugin>>>,
    loader: Box<dyn PluginLoader>,
    config: RegistryConfig,
    stats: RegistryStats,
}

/// Information about a registered plugin.
struct RegisteredPlugin {
    plugin: Arc<tokio::sync::RwLock<Box<dyn Plugin>>>,
    metadata: PluginMetadata,
    registered_at: SystemTime,
    last_accessed: Option<SystemTime>,
    access_count: u64,
}

/// Plugin registry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Maximum number of plugins that can be registered.
    pub max_plugins: usize,
    /// Whether to enable plugin health monitoring.
    pub enable_health_monitoring: bool,
    /// Health check interval in seconds.
    pub health_check_interval_seconds: u64,
    /// Whether to automatically unload unhealthy plugins.
    pub auto_unload_unhealthy: bool,
    /// Plugin access timeout in seconds.
    pub access_timeout_seconds: u64,
}

/// Plugin registry statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    /// Number of plugins currently registered.
    pub registered_plugins: u64,
    /// Total number of plugin registrations.
    pub total_registrations: u64,
    /// Total number of plugin unregistrations.
    pub total_unregistrations: u64,
    /// Number of plugin access operations.
    pub plugin_accesses: u64,
    /// Number of failed plugin accesses.
    pub failed_accesses: u64,
    /// Registry uptime in seconds.
    pub uptime_seconds: u64,
    /// Last health check timestamp.
    pub last_health_check: Option<SystemTime>,
}

impl DefaultPluginRegistry {
    /// Creates a new plugin registry with the specified configuration.
    pub fn new(config: RegistryConfig, loader_config: LoaderConfig) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            loader: Box::new(DefaultPluginLoader::new(loader_config)),
            config,
            stats: RegistryStats::new(),
        }
    }

    /// Creates a plugin registry with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(RegistryConfig::default(), LoaderConfig::default())
    }

    /// Checks if the maximum number of plugins is reached.
    fn check_plugin_limit(&self) -> Result<()> {
        let plugins = self.plugins.read()
            .map_err(|_| Error::Plugin(PluginError::CommunicationError {
                plugin: "registry".to_string(),
                message: "Failed to acquire read lock".to_string(),
            }))?;
        let plugin_count = plugins.len();

        if plugin_count >= self.config.max_plugins {
            return Err(Error::Plugin(PluginError::ConfigurationError {
                plugin: "registry".to_string(),
                message: format!("Maximum number of plugins ({}) reached", self.config.max_plugins),
            }));
        }

        Ok(())
    }

    /// Updates plugin access statistics.
    fn update_access_stats(&mut self, plugin_id: &str, success: bool) {
        self.stats.plugin_accesses += 1;
        if !success {
            self.stats.failed_accesses += 1;
        }

        // Update plugin-specific access stats
        if let Ok(mut plugins) = self.plugins.write() {
            if let Some(registered_plugin) = plugins.get_mut(plugin_id) {
                registered_plugin.last_accessed = Some(SystemTime::now());
                if success {
                    registered_plugin.access_count += 1;
                }
            }
        }
    }

    /// Performs health checks on all registered plugins.
    async fn perform_health_checks(&mut self) -> Result<()> {
        if !self.config.enable_health_monitoring {
            return Ok(());
        }

        let plugin_ids: Vec<String> = {
            let plugins = self.plugins.read()
                .map_err(|_| Error::Plugin(PluginError::CommunicationError {
                    plugin: "registry".to_string(),
                    message: "Failed to acquire read lock".to_string(),
                }))?;
            plugins.keys().cloned().collect()
        };

        let mut unhealthy_plugins: Vec<String> = Vec::new();

        // TODO: Implement health check functionality when Plugin trait supports it
        // For now, skip health checks since Plugin trait doesn't have health_check method

        // Unload unhealthy plugins
        for plugin_id in unhealthy_plugins {
            if let Err(e) = self.unregister_plugin(&plugin_id).await {
                eprintln!("Failed to unload unhealthy plugin {}: {}", plugin_id, e);
            }
        }

        self.stats.last_health_check = Some(SystemTime::now());
        Ok(())
    }
}

#[async_trait]
impl PluginRegistry for DefaultPluginRegistry {
    async fn register_plugin(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        // Check plugin limit
        self.check_plugin_limit()?;

        let metadata = plugin.metadata().clone();
        let plugin_id = metadata.id.clone();

        // Check if plugin is already registered
        {
            let plugins = self.plugins.read()
                .map_err(|_| Error::Plugin(PluginError::CommunicationError {
                    plugin: "registry".to_string(),
                    message: "Failed to acquire read lock".to_string(),
                }))?;
            
            if plugins.contains_key(&plugin_id) {
                return Err(Error::Plugin(PluginError::ConfigurationError {
                    plugin: plugin_id.clone(),
                    message: "Plugin is already registered".to_string(),
                }));
            }
        }

        // Create registered plugin entry
        let registered_plugin = RegisteredPlugin {
            plugin: Arc::new(tokio::sync::RwLock::new(plugin)),
            metadata,
            registered_at: SystemTime::now(),
            last_accessed: None,
            access_count: 0,
        };

        // Add to registry
        {
            let mut plugins = self.plugins.write()
                .map_err(|_| Error::Plugin(PluginError::CommunicationError {
                    plugin: "registry".to_string(),
                    message: "Failed to acquire write lock for registration".to_string(),
                }))?;
            plugins.insert(plugin_id, registered_plugin);
        }

        // Update statistics
        self.stats.registered_plugins += 1;
        self.stats.total_registrations += 1;

        Ok(())
    }

    async fn unregister_plugin(&mut self, plugin_id: &str) -> Result<()> {
        let registered_plugin = {
            let mut plugins = self.plugins.write()
                .map_err(|_| Error::Plugin(PluginError::CommunicationError {
                    plugin: "registry".to_string(),
                    message: "Failed to acquire write lock".to_string(),
                }))?;
            
            plugins.remove(plugin_id)
                .ok_or_else(|| Error::Plugin(PluginError::NotFoundError {
                    plugin: plugin_id.to_string(),
                    message: format!("Plugin {} not found", plugin_id),
                }))?
        };

        // Shutdown the plugin - simplified approach
        let plugin_arc = registered_plugin.plugin.clone();
        let mut plugin = plugin_arc.write().await;
        plugin.shutdown().await?;

        // Update statistics
        self.stats.registered_plugins = self.stats.registered_plugins.saturating_sub(1);
        self.stats.total_unregistrations += 1;

        Ok(())
    }

    fn get_plugin(&self, plugin_id: &str) -> Result<Arc<tokio::sync::RwLock<Box<dyn Plugin>>>> {
        let plugins = self.plugins.read()
            .map_err(|_| Error::Plugin(PluginError::CommunicationError {
                plugin: "registry".to_string(),
                message: "Failed to acquire read lock".to_string(),
            }))?;
        
        let registered_plugin = plugins.get(plugin_id)
            .ok_or_else(|| Error::Plugin(PluginError::NotFoundError {
                plugin: plugin_id.to_string(),
                message: format!("Plugin {} not found", plugin_id),
            }))?;

        Ok(registered_plugin.plugin.clone())
    }

    fn list_plugins(&self) -> Vec<String> {
        self.plugins.read()
            .map(|plugins| plugins.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn find_plugins_for_survey(&self, survey_code: &str) -> Vec<String> {
        let plugins = match self.plugins.read() {
            Ok(plugins) => plugins,
            Err(_) => return vec![],
        };

        plugins.iter()
            .filter(|(_, registered_plugin)| {
                registered_plugin.metadata.supported_surveys
                    .iter()
                    .any(|code| code.eq_ignore_ascii_case(survey_code))
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    fn get_plugin_metadata(&self, plugin_id: &str) -> Result<PluginMetadata> {
        let plugins = self.plugins.read()
            .map_err(|_| Error::Plugin(PluginError::CommunicationError {
                plugin: "registry".to_string(),
                message: "Failed to acquire read lock".to_string(),
            }))?;
        
        let registered_plugin = plugins.get(plugin_id)
            .ok_or_else(|| Error::Plugin(PluginError::NotFoundError {
                plugin: plugin_id.to_string(),
                message: format!("Plugin {} not found", plugin_id),
            }))?;

        Ok(registered_plugin.metadata.clone())
    }

    fn is_plugin_registered(&self, plugin_id: &str) -> bool {
        self.plugins.read()
            .map(|plugins| plugins.contains_key(plugin_id))
            .unwrap_or(false)
    }

    fn registry_stats(&self) -> RegistryStats {
        self.stats.clone()
    }

    async fn clear(&mut self) -> Result<()> {
        let plugin_ids = self.list_plugins();
        
        for plugin_id in plugin_ids {
            if let Err(e) = self.unregister_plugin(&plugin_id).await {
                eprintln!("Failed to unregister plugin {}: {}", plugin_id, e);
            }
        }

        Ok(())
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_plugins: 100,
            enable_health_monitoring: true,
            health_check_interval_seconds: 300, // 5 minutes
            auto_unload_unhealthy: false,
            access_timeout_seconds: 30,
        }
    }
}

impl RegistryStats {
    /// Creates a new RegistryStats instance with default values.
    pub fn new() -> Self {
        Self {
            registered_plugins: 0,
            total_registrations: 0,
            total_unregistrations: 0,
            plugin_accesses: 0,
            failed_accesses: 0,
            uptime_seconds: 0,
            last_health_check: None,
        }
    }

    /// Returns the success rate for plugin accesses as a percentage.
    pub fn access_success_rate(&self) -> f64 {
        if self.plugin_accesses == 0 {
            0.0
        } else {
            let successful_accesses = self.plugin_accesses - self.failed_accesses;
            (successful_accesses as f64 / self.plugin_accesses as f64) * 100.0
        }
    }

    /// Returns the plugin turnover rate (unregistrations / registrations).
    pub fn turnover_rate(&self) -> f64 {
        if self.total_registrations == 0 {
            0.0
        } else {
            self.total_unregistrations as f64 / self.total_registrations as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::traits::PluginStats;
    use std::time::SystemTime;

    // Mock plugin for testing
    #[derive(Debug)]
struct MockPlugin {
        metadata: PluginMetadata,
        stats: PluginStats,
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn initialize(&mut self, _config: PluginConfig) -> Result<()> {
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<()> {
            Ok(())
        }

        fn validate_config(&self, _config: &PluginConfig) -> Result<()> {
            Ok(())
        }

        fn stats(&self) -> PluginStats {
            self.stats.clone()
        }
    }

    fn create_mock_plugin(id: &str, surveys: Vec<String>) -> Box<dyn Plugin> {
        Box::new(MockPlugin {
            metadata: PluginMetadata {
                id: id.to_string(),
                name: format!("Mock Plugin {}", id),
                version: "1.0.0".to_string(),
                description: "Mock plugin for testing".to_string(),
                author: "Test".to_string(),
                supported_surveys: surveys,
                created_at: SystemTime::now(),
            },
            stats: PluginStats {
                successful_operations: 0,
                failed_operations: 0,
                total_processing_time_ms: 0,
                records_processed: 0,
            },
        })
    }

    #[test]
    fn test_registry_config_default() {
        let config = RegistryConfig::default();
        assert_eq!(config.max_plugins, 100);
        assert!(config.enable_health_monitoring);
        assert_eq!(config.health_check_interval_seconds, 300);
        assert!(!config.auto_unload_unhealthy);
    }

    #[test]
    fn test_registry_stats_new() {
        let stats = RegistryStats::new();
        assert_eq!(stats.registered_plugins, 0);
        assert_eq!(stats.access_success_rate(), 0.0);
        assert_eq!(stats.turnover_rate(), 0.0);
    }

    #[test]
    fn test_registry_stats_calculations() {
        let mut stats = RegistryStats::new();
        stats.plugin_accesses = 10;
        stats.failed_accesses = 2;
        stats.total_registrations = 5;
        stats.total_unregistrations = 1;

        assert_eq!(stats.access_success_rate(), 80.0);
        assert_eq!(stats.turnover_rate(), 0.2);
    }

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = DefaultPluginRegistry::with_defaults();
        assert_eq!(registry.list_plugins().len(), 0);
        assert_eq!(registry.registry_stats().registered_plugins, 0);
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let mut registry = DefaultPluginRegistry::with_defaults();
        let plugin = create_mock_plugin("test_plugin", vec!["AP".to_string()]);

        let result = registry.register_plugin(plugin).await;
        assert!(result.is_ok());
        assert_eq!(registry.list_plugins().len(), 1);
        assert!(registry.is_plugin_registered("test_plugin"));
    }

    #[tokio::test]
    async fn test_duplicate_plugin_registration() {
        let mut registry = DefaultPluginRegistry::with_defaults();
        let plugin1 = create_mock_plugin("test_plugin", vec![]);
        let plugin2 = create_mock_plugin("test_plugin", vec![]);

        assert!(registry.register_plugin(plugin1).await.is_ok());
        assert!(registry.register_plugin(plugin2).await.is_err());
    }

    #[tokio::test]
    async fn test_find_plugins_for_survey() {
        let mut registry = DefaultPluginRegistry::with_defaults();
        let plugin1 = create_mock_plugin("plugin1", vec!["AP".to_string(), "BD".to_string()]);
        let plugin2 = create_mock_plugin("plugin2", vec!["CE".to_string()]);
        let plugin3 = create_mock_plugin("plugin3", vec!["AP".to_string()]);

        registry.register_plugin(plugin1).await.unwrap();
        registry.register_plugin(plugin2).await.unwrap();
        registry.register_plugin(plugin3).await.unwrap();

        let ap_plugins = registry.find_plugins_for_survey("AP");
        assert_eq!(ap_plugins.len(), 2);
        assert!(ap_plugins.contains(&"plugin1".to_string()));
        assert!(ap_plugins.contains(&"plugin3".to_string()));

        let ce_plugins = registry.find_plugins_for_survey("CE");
        assert_eq!(ce_plugins.len(), 1);
        assert!(ce_plugins.contains(&"plugin2".to_string()));
    }

    #[tokio::test]
    async fn test_plugin_unregistration() {
        let mut registry = DefaultPluginRegistry::with_defaults();
        let plugin = create_mock_plugin("test_plugin", vec![]);

        registry.register_plugin(plugin).await.unwrap();
        assert!(registry.is_plugin_registered("test_plugin"));

        let result = registry.unregister_plugin("test_plugin").await;
        assert!(result.is_ok());
        assert!(!registry.is_plugin_registered("test_plugin"));
        assert_eq!(registry.list_plugins().len(), 0);
    }
}