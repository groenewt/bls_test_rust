//! # Output Registry Implementation
//!
//! This module provides registry implementations for managing and discovering
//! output generators and format writers.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::types::{ProcessingError, Result};
use crate::output::traits::{OutputGenerator, OutputRegistry};

/// Default implementation of the output registry
pub struct DefaultOutputRegistry {
    /// Map of generator name to generator instance
    generators: Arc<Mutex<HashMap<String, Box<dyn OutputGenerator>>>>,
    /// Registry metadata
    metadata: HashMap<String, String>,
}

impl DefaultOutputRegistry {
    /// Create a new output registry
    pub fn new() -> Self {
        Self {
            generators: Arc::new(Mutex::new(HashMap::new())),
            metadata: HashMap::new(),
        }
    }

    /// Get registry metadata
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Set registry metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get generator count
    pub fn generator_count(&self) -> Result<usize> {
        let generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;
        Ok(generators.len())
    }
}

impl Default for DefaultOutputRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRegistry for DefaultOutputRegistry {
    fn register_generator(
        &mut self,
        name: String,
        generator: Box<dyn OutputGenerator>,
    ) -> Result<()> {
        let mut generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        if generators.contains_key(&name) {
            return Err(ProcessingError::invalid_configuration(
                format!("Generator '{name}' is already registered"),
            ));
        }

        generators.insert(name, generator);
        Ok(())
    }

    fn unregister_generator(&mut self, name: &str) -> Result<()> {
        let mut generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        if generators.remove(name).is_none() {
            return Err(ProcessingError::invalid_configuration(format!(
                "Generator '{name}' is not registered"
            )));
        }

        Ok(())
    }

    fn get_generator(&self, name: &str) -> Result<&dyn OutputGenerator> {
        // Note: This implementation has lifetime issues with the mutex guard
        // In a real implementation, you'd need to use Arc<dyn OutputGenerator> or similar
        Err(ProcessingError::resource_exhausted(
            "Direct generator access not supported in this implementation".to_string(),
        ))
    }

    fn get_generator_mut(&mut self, name: &str) -> Result<&mut dyn OutputGenerator> {
        // Note: This implementation has lifetime issues with the mutex guard
        // In a real implementation, you'd need to use Arc<Mutex<dyn OutputGenerator>> or similar
        Err(ProcessingError::resource_exhausted(
            "Direct mutable generator access not supported in this implementation".to_string(),
        ))
    }

    fn list_generators(&self) -> Vec<String> {
        if let Ok(generators) = self.generators.lock() {
            generators.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    fn has_generator(&self, name: &str) -> bool {
        if let Ok(generators) = self.generators.lock() {
            generators.contains_key(name)
        } else {
            false
        }
    }

    fn get_generator_for_format(&self, format: &str) -> Result<&dyn OutputGenerator> {
        // This would need to iterate through generators and find one that supports the format
        // For now, return an error indicating this is not implemented
        Err(ProcessingError::resource_exhausted(format!(
            "Finding generator for format '{format}' not implemented"
        )))
    }

    fn clear(&mut self) {
        if let Ok(mut generators) = self.generators.lock() {
            generators.clear();
        }
    }
}

/// Enhanced registry implementation with additional features
pub struct OutputRegistryImpl {
    /// Thread-safe generator storage
    generators: Arc<Mutex<HashMap<String, GeneratorInfo>>>,
    /// Registry configuration
    config: RegistryConfig,
}

/// Information about a registered generator
#[derive(Debug, Clone)]
pub struct GeneratorInfo {
    /// Generator name
    pub name: String,
    /// Generator description
    pub description: String,
    /// Supported formats
    pub supported_formats: Vec<String>,
    /// Registration timestamp
    pub registered_at: std::time::SystemTime,
    /// Usage count
    pub usage_count: u64,
}

/// Configuration for the output registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Maximum number of generators
    pub max_generators: usize,
    /// Enable usage tracking
    pub track_usage: bool,
    /// Enable automatic cleanup
    pub auto_cleanup: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_generators: 50,
            track_usage: true,
            auto_cleanup: false,
        }
    }
}

impl OutputRegistryImpl {
    /// Create a new output registry with configuration
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            generators: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Create a new output registry with default configuration
    pub fn default() -> Self {
        Self::new(RegistryConfig::default())
    }

    /// Get registry configuration
    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }

    /// Get generator information
    pub fn get_generator_info(&self, name: &str) -> Result<Option<GeneratorInfo>> {
        let generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        Ok(generators.get(name).cloned())
    }

    /// List all generator information
    pub fn list_generator_info(&self) -> Result<Vec<GeneratorInfo>> {
        let generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        Ok(generators.values().cloned().collect())
    }

    /// Update generator usage count
    pub fn increment_usage(&self, name: &str) -> Result<()> {
        if !self.config.track_usage {
            return Ok(());
        }

        let mut generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        if let Some(info) = generators.get_mut(name) {
            info.usage_count += 1;
        }

        Ok(())
    }

    /// Get generators by format
    pub fn get_generators_by_format(&self, format: &str) -> Result<Vec<String>> {
        let generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        let matching_generators = generators
            .values()
            .filter(|info| info.supported_formats.contains(&format.to_lowercase()))
            .map(|info| info.name.clone())
            .collect();

        Ok(matching_generators)
    }

    /// Register generator information
    pub fn register_generator_info(&self, info: GeneratorInfo) -> Result<()> {
        let mut generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        if generators.len() >= self.config.max_generators {
            return Err(ProcessingError::resource_exhausted(format!(
                "Registry is full (max: {})",
                self.config.max_generators
            )));
        }

        if generators.contains_key(&info.name) {
            return Err(ProcessingError::invalid_configuration(
                format!("Generator '{}' is already registered", info.name),
            ));
        }

        generators.insert(info.name.clone(), info);
        Ok(())
    }

    /// Unregister generator information
    pub fn unregister_generator_info(&self, name: &str) -> Result<()> {
        let mut generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        if generators.remove(name).is_none() {
            return Err(ProcessingError::invalid_configuration(
                format!("Generator '{name}' is not registered"),
            ));
        }

        Ok(())
    }

    /// Clear all generator information
    pub fn clear_all(&self) -> Result<()> {
        let mut generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        generators.clear();
        Ok(())
    }

    /// Get registry statistics
    pub fn get_statistics(&self) -> Result<RegistryStatistics> {
        let generators = self.generators.lock().map_err(|e| {
            ProcessingError::system_error(format!("Failed to lock generators: {e}"))
        })?;

        let total_generators = generators.len();
        let total_usage = generators.values().map(|info| info.usage_count).sum();

        let mut format_counts = HashMap::new();
        for info in generators.values() {
            for format in &info.supported_formats {
                *format_counts.entry(format.clone()).or_insert(0) += 1;
            }
        }

        Ok(RegistryStatistics {
            total_generators,
            total_usage,
            format_counts,
        })
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStatistics {
    /// Total number of registered generators
    pub total_generators: usize,
    /// Total usage count across all generators
    pub total_usage: u64,
    /// Count of generators by format
    pub format_counts: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::format::CsvOutputGenerator;

    #[test]
    fn test_default_registry_creation() {
        let registry = DefaultOutputRegistry::new();
        assert_eq!(registry.list_generators().len(), 0);
    }

    #[test]
    fn test_output_registry_impl_creation() {
        let config = RegistryConfig::default();
        let registry = OutputRegistryImpl::new(config);
        assert_eq!(registry.config().max_generators, 50);
    }

    #[test]
    fn test_register_generator_info() {
        let registry = OutputRegistryImpl::default();

        let info = GeneratorInfo {
            name: "test_generator".to_string(),
            description: "Test generator".to_string(),
            supported_formats: vec!["csv".to_string()],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };

        assert!(registry.register_generator_info(info).is_ok());
        assert!(
            registry
                .get_generator_info("test_generator")
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn test_register_duplicate_generator_info() {
        let registry = OutputRegistryImpl::default();

        let info1 = GeneratorInfo {
            name: "test_generator".to_string(),
            description: "Test generator 1".to_string(),
            supported_formats: vec!["csv".to_string()],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };

        let info2 = GeneratorInfo {
            name: "test_generator".to_string(),
            description: "Test generator 2".to_string(),
            supported_formats: vec!["json".to_string()],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };

        assert!(registry.register_generator_info(info1).is_ok());
        assert!(registry.register_generator_info(info2).is_err());
    }

    #[test]
    fn test_get_generators_by_format() {
        let registry = OutputRegistryImpl::default();

        let info1 = GeneratorInfo {
            name: "csv_generator".to_string(),
            description: "CSV generator".to_string(),
            supported_formats: vec!["csv".to_string()],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };

        let info2 = GeneratorInfo {
            name: "multi_generator".to_string(),
            description: "Multi-format generator".to_string(),
            supported_formats: vec!["csv".to_string(), "json".to_string()],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };

        registry.register_generator_info(info1).unwrap();
        registry.register_generator_info(info2).unwrap();

        let csv_generators = registry.get_generators_by_format("csv").unwrap();
        assert_eq!(csv_generators.len(), 2);

        let json_generators = registry.get_generators_by_format("json").unwrap();
        assert_eq!(json_generators.len(), 1);
    }

    #[test]
    fn test_increment_usage() {
        let registry = OutputRegistryImpl::default();

        let info = GeneratorInfo {
            name: "test_generator".to_string(),
            description: "Test generator".to_string(),
            supported_formats: vec!["csv".to_string()],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };

        registry.register_generator_info(info).unwrap();

        assert!(registry.increment_usage("test_generator").is_ok());

        let updated_info = registry
            .get_generator_info("test_generator")
            .unwrap()
            .unwrap();
        assert_eq!(updated_info.usage_count, 1);
    }

    #[test]
    fn test_registry_statistics() {
        let registry = OutputRegistryImpl::default();

        let info1 = GeneratorInfo {
            name: "generator1".to_string(),
            description: "Generator 1".to_string(),
            supported_formats: vec!["csv".to_string()],
            registered_at: std::time::SystemTime::now(),
            usage_count: 5,
        };

        let info2 = GeneratorInfo {
            name: "generator2".to_string(),
            description: "Generator 2".to_string(),
            supported_formats: vec!["json".to_string()],
            registered_at: std::time::SystemTime::now(),
            usage_count: 3,
        };

        registry.register_generator_info(info1).unwrap();
        registry.register_generator_info(info2).unwrap();

        let stats = registry.get_statistics().unwrap();
        assert_eq!(stats.total_generators, 2);
        assert_eq!(stats.total_usage, 8);
        assert_eq!(stats.format_counts.get("csv"), Some(&1));
        assert_eq!(stats.format_counts.get("json"), Some(&1));
    }
}
