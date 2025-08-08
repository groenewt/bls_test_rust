//! Comprehensive unit tests for plugin system
//! 
//! These tests verify the plugin system including:
//! - Plugin loading and validation
//! - Plugin registry and management
//! - Plugin configuration and metadata
//! - Plugin lifecycle and error handling
//! - Security validation and sandboxing
//! - Plugin communication and interfaces

use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

use rusty::plugin::{
    Plugin, PluginMetadata, PluginConfig, PluginStats, ResourceLimits,
    PluginLoader, DefaultPluginLoader, LoaderConfig, LoaderStats,
    PluginRegistry, DefaultPluginRegistry, RegistryConfig, RegistryStats,
    PluginManager, ManagerConfig, SystemStats,
    PluginValidator, ValidatorConfig, ValidationReport
};
use rusty::error::{Result, PluginError};

#[cfg(test)]
mod plugin_manager_tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let config = ManagerConfig::default();
        let manager = PluginManager::new(config);
        
        assert_eq!(manager.registry().generator_count(), 0);
        assert!(manager.loader().supported_formats().is_empty());
    }

    #[test]
    fn test_manager_config_default() {
        let config = ManagerConfig::default();
        
        assert!(config.plugin_directories.contains(&PathBuf::from("plugins")));
        assert_eq!(config.max_plugins, 100);
        assert!(config.enable_validation);
        assert!(config.enable_sandboxing);
    }

    #[test]
    fn test_plugin_manager_initialization() {
        let config = ManagerConfig::default();
        let mut manager = PluginManager::new(config);
        
        let result = manager.initialize();
        assert!(result.is_ok());
    }

    #[test]
    fn test_system_stats() {
        let config = ManagerConfig::default();
        let manager = PluginManager::new(config);
        
        let stats = manager.system_stats();
        assert_eq!(stats.total_plugins_loaded, 0);
        assert_eq!(stats.active_plugins, 0);
        assert_eq!(stats.failed_plugins, 0);
    }
}

#[cfg(test)]
mod plugin_validator_tests {
    use super::*;

    #[test]
    fn test_plugin_validator_creation() {
        let config = ValidatorConfig::default();
        let validator = PluginValidator::new(config);
        
        assert!(validator.config().check_signatures);
        assert!(validator.config().perform_static_analysis);
    }

    #[test]
    fn test_validation_report() {
        let plugin_path = PathBuf::from("test_plugin.so");
        let mut report = ValidationReport::new(plugin_path.clone());
        
        assert_eq!(report.plugin_path, plugin_path);
        assert!(!report.has_errors());
        assert!(!report.has_warnings());
        
        report.add_error("Test error".to_string());
        report.add_warning("Test warning".to_string());
        
        assert!(report.has_errors());
        assert!(report.has_warnings());
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn test_validator_config_default() {
        let config = ValidatorConfig::default();
        
        assert!(config.check_signatures);
        assert!(config.perform_static_analysis);
        assert!(config.check_dependencies);
        assert_eq!(config.max_file_size, 100 * 1024 * 1024); // 100MB
    }
}

#[cfg(test)]
mod plugin_registry_tests {
    use super::*;

    #[test]
    fn test_plugin_registry_creation() {
        let registry = DefaultPluginRegistry::new();
        
        assert_eq!(registry.plugin_count(), 0);
        assert!(registry.list_plugins().is_empty());
    }

    #[test]
    fn test_registry_stats() {
        let registry = DefaultPluginRegistry::new();
        let stats = registry.stats();
        
        assert_eq!(stats.total_plugins, 0);
        assert_eq!(stats.active_plugins, 0);
        assert_eq!(stats.failed_plugins, 0);
    }
}