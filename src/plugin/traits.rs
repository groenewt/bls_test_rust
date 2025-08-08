//! Plugin system trait definitions for the Rusty BLS Data Processing system.

use std::collections::HashMap;
use std::time::SystemTime;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::data::model::{Series, Observation, Lookup, Survey};
use crate::processing::traits::ProcessedData;
use crate::config::model::ConfigValue;
use crate::error::types::Result;

/// Core plugin trait that all plugins must implement.
#[async_trait]
pub trait Plugin: Send + Sync + std::fmt::Debug {
    /// Returns the plugin's metadata information.
    fn metadata(&self) -> &PluginMetadata;

    /// Returns the plugin's unique identifier.
    fn id(&self) -> &str {
        &self.metadata().id
    }

    /// Initializes the plugin with the provided configuration.
    async fn initialize(&mut self, config: PluginConfig) -> Result<()>;

    /// Shuts down the plugin and cleans up resources.
    async fn shutdown(&mut self) -> Result<()>;

    /// Validates the plugin's configuration.
    fn validate_config(&self, config: &PluginConfig) -> Result<()>;

    /// Returns the plugin's runtime statistics.
    fn stats(&self) -> PluginStats;
}

/// Plugin metadata containing information about the plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub supported_surveys: Vec<String>,
    pub created_at: SystemTime,
}

/// Plugin configuration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub parameters: HashMap<String, ConfigValue>,
    pub resource_limits: ResourceLimits,
}

/// Plugin runtime statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStats {
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub total_processing_time_ms: u64,
    pub records_processed: u64,
}

/// Resource limits for plugin execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_bytes: u64,
    pub max_cpu_percent: f64,
    pub max_execution_time_seconds: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_cpu_percent: 80.0,
            max_execution_time_seconds: 3600, // 1 hour
        }
    }
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            parameters: HashMap::new(),
            resource_limits: ResourceLimits::default(),
        }
    }
}