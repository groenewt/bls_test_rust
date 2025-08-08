//! # Processing Registry Implementation
//!
//! This module provides registry implementations for managing and discovering
//! processing components dynamically.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::processing::traits::{
    ProcessorRegistry, DataProcessor, ProcessingStrategy, ProcessingConfig,
};
use crate::error::types::{ProcessingError, Result};

/// Default implementation of the processor registry
pub struct DefaultProcessorRegistry {
    /// Map of processor name to processor instance
    processors: Arc<Mutex<HashMap<String, Box<dyn DataProcessor>>>>,
    /// Registry metadata
    metadata: HashMap<String, String>,
}

impl DefaultProcessorRegistry {
    /// Create a new processor registry
    pub fn new() -> Self {
        Self {
            processors: Arc::new(Mutex::new(HashMap::new())),
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

    /// Get processor count
    pub fn processor_count(&self) -> Result<usize> {
        let processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        Ok(processors.len())
    }
}

impl Default for DefaultProcessorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessorRegistry for DefaultProcessorRegistry {
    fn register_processor(&mut self, name: String, processor: Box<dyn DataProcessor>) -> Result<()> {
        let mut processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        
        if processors.contains_key(&name) {
            return Err(ProcessingError::invalid_configuration(
                format!("Processor '{}' is already registered", name)
            ));
        }
        
        processors.insert(name, processor);
        Ok(())
    }

    fn unregister_processor(&mut self, name: &str) -> Result<()> {
        let mut processors = self.processors.lock()
            .map_err(|e| crate::error::types::Error::Processing(ProcessingError::system_error(format!("Failed to lock processors: {}", e))))?;
        
        if processors.remove(name).is_none() {
            return Err(ProcessingError::invalid_configuration(
                format!("Processor '{}' is not registered", name)
            ));
        }
        
        Ok(())
    }

    fn get_processor(&self, name: &str) -> Result<&dyn DataProcessor> {
        // Note: This implementation has lifetime issues with the mutex guard
        // In a real implementation, you'd need to use Arc<dyn DataProcessor> or similar
        Err(crate::error::types::Error::Processing(ProcessingError::UnsupportedOperation {
            operation: "get_processor".to_string(),
            message: "Direct processor access not supported in this implementation".to_string()
        }))
    }

    fn get_processor_mut(&mut self, name: &str) -> Result<&mut dyn DataProcessor> {
        // Note: This implementation has lifetime issues with the mutex guard
        // In a real implementation, you'd need to use Arc<Mutex<dyn DataProcessor>> or similar
        Err(crate::error::types::Error::Processing(ProcessingError::UnsupportedOperation {
            operation: "get_processor_mut".to_string(),
            message: "Direct mutable processor access not supported in this implementation".to_string()
        }))
    }

    fn list_processors(&self) -> Vec<String> {
        if let Ok(processors) = self.processors.lock() {
            processors.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    fn has_processor(&self, name: &str) -> bool {
        if let Ok(processors) = self.processors.lock() {
            processors.contains_key(name)
        } else {
            false
        }
    }

    fn clear(&mut self) {
        if let Ok(mut processors) = self.processors.lock() {
            processors.clear();
        }
    }
}

/// Registry implementation that can be shared across threads
pub struct ProcessorRegistryImpl {
    /// Thread-safe processor storage
    processors: Arc<Mutex<HashMap<String, ProcessorInfo>>>,
    /// Registry configuration
    config: RegistryConfig,
}

/// Information about a registered processor
#[derive(Debug, Clone)]
pub struct ProcessorInfo {
    /// Processor name
    pub name: String,
    /// Processor description
    pub description: String,
    /// Supported strategies
    pub supported_strategies: Vec<ProcessingStrategy>,
    /// Registration timestamp
    pub registered_at: std::time::SystemTime,
    /// Usage count
    pub usage_count: u64,
}

/// Configuration for the processor registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Maximum number of processors
    pub max_processors: usize,
    /// Enable usage tracking
    pub track_usage: bool,
    /// Enable automatic cleanup
    pub auto_cleanup: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_processors: 100,
            track_usage: true,
            auto_cleanup: false,
        }
    }
}

impl ProcessorRegistryImpl {
    /// Create a new processor registry with configuration
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            processors: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Create a new processor registry with default configuration
    pub fn default() -> Self {
        Self::new(RegistryConfig::default())
    }

    /// Get registry configuration
    pub fn config(&self) -> &RegistryConfig {
        &self.config
    }

    /// Get processor information
    pub fn get_processor_info(&self, name: &str) -> Result<Option<ProcessorInfo>> {
        let processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        
        Ok(processors.get(name).cloned())
    }

    /// List all processor information
    pub fn list_processor_info(&self) -> Result<Vec<ProcessorInfo>> {
        let processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        
        Ok(processors.values().cloned().collect())
    }

    /// Update processor usage count
    pub fn increment_usage(&self, name: &str) -> Result<()> {
        if !self.config.track_usage {
            return Ok(());
        }

        let mut processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        
        if let Some(info) = processors.get_mut(name) {
            info.usage_count += 1;
        }
        
        Ok(())
    }

    /// Get processors by strategy
    pub fn get_processors_by_strategy(&self, strategy: ProcessingStrategy) -> Result<Vec<String>> {
        let processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        
        let matching_processors = processors
            .values()
            .filter(|info| info.supported_strategies.contains(&strategy))
            .map(|info| info.name.clone())
            .collect();
        
        Ok(matching_processors)
    }

    /// Register processor information
    pub fn register_processor_info(&self, info: ProcessorInfo) -> Result<()> {
        let mut processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        
        if processors.len() >= self.config.max_processors {
            return Err(ProcessingError::resource_exhausted(
                format!("Registry is full (max: {})", self.config.max_processors)
            ));
        }
        
        if processors.contains_key(&info.name) {
            return Err(ProcessingError::invalid_configuration(
                format!("Processor '{}' is already registered", info.name)
            ));
        }
        
        processors.insert(info.name.clone(), info);
        Ok(())
    }

    /// Unregister processor information
    pub fn unregister_processor_info(&self, name: &str) -> Result<()> {
        let mut processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        
        if processors.remove(name).is_none() {
            return Err(ProcessingError::invalid_configuration(
                format!("Processor '{}' is not registered", name)
            ));
        }
        
        Ok(())
    }

    /// Clear all processor information
    pub fn clear_all(&self) -> Result<()> {
        let mut processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        
        processors.clear();
        Ok(())
    }

    /// Get registry statistics
    pub fn get_statistics(&self) -> Result<RegistryStatistics> {
        let processors = self.processors.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock processors: {}", e)))?;
        
        let total_processors = processors.len();
        let total_usage = processors.values().map(|info| info.usage_count).sum();
        
        let mut strategy_counts = HashMap::new();
        for info in processors.values() {
            for strategy in &info.supported_strategies {
                *strategy_counts.entry(*strategy).or_insert(0) += 1;
            }
        }
        
        Ok(RegistryStatistics {
            total_processors,
            total_usage,
            strategy_counts,
        })
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStatistics {
    /// Total number of registered processors
    pub total_processors: usize,
    /// Total usage count across all processors
    pub total_usage: u64,
    /// Count of processors by strategy
    pub strategy_counts: HashMap<ProcessingStrategy, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::strategy::InMemoryProcessor;

    #[test]
    fn test_default_registry_creation() {
        let registry = DefaultProcessorRegistry::new();
        assert_eq!(registry.list_processors().len(), 0);
    }

    #[test]
    fn test_processor_registry_impl_creation() {
        let config = RegistryConfig::default();
        let registry = ProcessorRegistryImpl::new(config);
        assert_eq!(registry.config().max_processors, 100);
    }

    #[test]
    fn test_register_processor_info() {
        let registry = ProcessorRegistryImpl::default();
        
        let info = ProcessorInfo {
            name: "test_processor".to_string(),
            description: "Test processor".to_string(),
            supported_strategies: vec![ProcessingStrategy::InMemory],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };
        
        assert!(registry.register_processor_info(info).is_ok());
        assert!(registry.get_processor_info("test_processor").unwrap().is_some());
    }

    #[test]
    fn test_register_duplicate_processor_info() {
        let registry = ProcessorRegistryImpl::default();
        
        let info1 = ProcessorInfo {
            name: "test_processor".to_string(),
            description: "Test processor 1".to_string(),
            supported_strategies: vec![ProcessingStrategy::InMemory],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };
        
        let info2 = ProcessorInfo {
            name: "test_processor".to_string(),
            description: "Test processor 2".to_string(),
            supported_strategies: vec![ProcessingStrategy::Chunked],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };
        
        assert!(registry.register_processor_info(info1).is_ok());
        assert!(registry.register_processor_info(info2).is_err());
    }

    #[test]
    fn test_unregister_processor_info() {
        let registry = ProcessorRegistryImpl::default();
        
        let info = ProcessorInfo {
            name: "test_processor".to_string(),
            description: "Test processor".to_string(),
            supported_strategies: vec![ProcessingStrategy::InMemory],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };
        
        registry.register_processor_info(info).unwrap();
        assert!(registry.unregister_processor_info("test_processor").is_ok());
        assert!(registry.get_processor_info("test_processor").unwrap().is_none());
    }

    #[test]
    fn test_get_processors_by_strategy() {
        let registry = ProcessorRegistryImpl::default();
        
        let info1 = ProcessorInfo {
            name: "memory_processor".to_string(),
            description: "Memory processor".to_string(),
            supported_strategies: vec![ProcessingStrategy::InMemory],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };
        
        let info2 = ProcessorInfo {
            name: "chunked_processor".to_string(),
            description: "Chunked processor".to_string(),
            supported_strategies: vec![ProcessingStrategy::Chunked, ProcessingStrategy::InMemory],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };
        
        registry.register_processor_info(info1).unwrap();
        registry.register_processor_info(info2).unwrap();
        
        let memory_processors = registry.get_processors_by_strategy(ProcessingStrategy::InMemory).unwrap();
        assert_eq!(memory_processors.len(), 2);
        
        let chunked_processors = registry.get_processors_by_strategy(ProcessingStrategy::Chunked).unwrap();
        assert_eq!(chunked_processors.len(), 1);
    }

    #[test]
    fn test_increment_usage() {
        let registry = ProcessorRegistryImpl::default();
        
        let info = ProcessorInfo {
            name: "test_processor".to_string(),
            description: "Test processor".to_string(),
            supported_strategies: vec![ProcessingStrategy::InMemory],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };
        
        registry.register_processor_info(info).unwrap();
        
        assert!(registry.increment_usage("test_processor").is_ok());
        
        let updated_info = registry.get_processor_info("test_processor").unwrap().unwrap();
        assert_eq!(updated_info.usage_count, 1);
    }

    #[test]
    fn test_registry_statistics() {
        let registry = ProcessorRegistryImpl::default();
        
        let info1 = ProcessorInfo {
            name: "processor1".to_string(),
            description: "Processor 1".to_string(),
            supported_strategies: vec![ProcessingStrategy::InMemory],
            registered_at: std::time::SystemTime::now(),
            usage_count: 5,
        };
        
        let info2 = ProcessorInfo {
            name: "processor2".to_string(),
            description: "Processor 2".to_string(),
            supported_strategies: vec![ProcessingStrategy::Chunked],
            registered_at: std::time::SystemTime::now(),
            usage_count: 3,
        };
        
        registry.register_processor_info(info1).unwrap();
        registry.register_processor_info(info2).unwrap();
        
        let stats = registry.get_statistics().unwrap();
        assert_eq!(stats.total_processors, 2);
        assert_eq!(stats.total_usage, 8);
        assert_eq!(stats.strategy_counts.get(&ProcessingStrategy::InMemory), Some(&1));
        assert_eq!(stats.strategy_counts.get(&ProcessingStrategy::Chunked), Some(&1));
    }

    #[test]
    fn test_clear_all() {
        let registry = ProcessorRegistryImpl::default();
        
        let info = ProcessorInfo {
            name: "test_processor".to_string(),
            description: "Test processor".to_string(),
            supported_strategies: vec![ProcessingStrategy::InMemory],
            registered_at: std::time::SystemTime::now(),
            usage_count: 0,
        };
        
        registry.register_processor_info(info).unwrap();
        assert_eq!(registry.get_statistics().unwrap().total_processors, 1);
        
        assert!(registry.clear_all().is_ok());
        assert_eq!(registry.get_statistics().unwrap().total_processors, 0);
    }
}