//! # Processing Strategy Factory
//!
//! This module provides factory implementations for creating and managing
//! processing strategies. It implements the ProcessorFactory trait and
//! provides utilities for strategy selection and processor creation.
//!
//! ## Features
//!
//! - **Automatic Strategy Selection**: Analyzes input data to recommend optimal strategy
//! - **Dynamic Processor Creation**: Creates processors based on strategy and configuration
//! - **Pipeline Factory**: Creates complete processing pipelines
//! - **Performance Optimization**: Considers system resources and data characteristics
//!
//! ## Usage
//!


use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::Error;
pub(crate) use crate::processing::traits::{
    DataProcessor, ProcessingPipeline, ProcessingStrategy,
    ProcessingConfig, ProcessingInput, ProcessingContext,
};
pub use crate::processing::traits::ProcessorFactory;
use crate::processing::strategy::{
    InMemoryProcessor, ChunkedProcessor, MemoryMappedProcessor,
    recommend_strategy as strategy_recommend,
};
use crate::processing::pipeline::DefaultPipeline;
use crate::error::types::{ProcessingError, Result};

/// Default implementation of the ProcessorFactory trait
pub struct DefaultProcessorFactory {
    /// Cache of created processors for reuse
    processor_cache: Arc<Mutex<HashMap<String, Box<dyn DataProcessor>>>>,
    /// Performance metrics for strategy selection
    performance_metrics: Arc<Mutex<HashMap<ProcessingStrategy, PerformanceMetrics>>>,
}

/// Performance metrics for a processing strategy
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    /// Average processing time per MB
    avg_time_per_mb: f64,
    /// Memory efficiency ratio
    memory_efficiency: f64,
    /// Success rate (0.0 to 1.0)
    success_rate: f64,
    /// Number of samples
    sample_count: u32,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            avg_time_per_mb: 1.0,
            memory_efficiency: 1.0,
            success_rate: 1.0,
            sample_count: 0,
        }
    }
}

impl DefaultProcessorFactory {
    /// Create a new processor factory
    pub fn new() -> Self {
        Self {
            processor_cache: Arc::new(Mutex::new(HashMap::new())),
            performance_metrics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a processor factory with initial performance metrics
    pub fn with_metrics(metrics: HashMap<ProcessingStrategy, PerformanceMetrics>) -> Self {
        Self {
            processor_cache: Arc::new(Mutex::new(HashMap::new())),
            performance_metrics: Arc::new(Mutex::new(metrics)),
        }
    }

    /// Update performance metrics for a strategy
    pub fn update_metrics(
        &self,
        strategy: ProcessingStrategy,
        processing_time: f64,
        data_size_mb: f64,
        memory_used_mb: f64,
        success: bool,
    ) -> Result<()> {
        let mut metrics = self.performance_metrics.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock metrics: {}", e)))?;

        let entry = metrics.entry(strategy).or_insert_with(PerformanceMetrics::default);
        
        // Update metrics using exponential moving average
        let alpha = 0.1; // Smoothing factor
        entry.avg_time_per_mb = entry.avg_time_per_mb * (1.0 - alpha) + (processing_time / data_size_mb) * alpha;
        entry.memory_efficiency = entry.memory_efficiency * (1.0 - alpha) + (data_size_mb / memory_used_mb) * alpha;
        entry.success_rate = entry.success_rate * (1.0 - alpha) + if success { 1.0 } else { 0.0 } * alpha;
        entry.sample_count += 1;

        Ok(())
    }

    /// Get performance metrics for a strategy
    pub fn get_metrics(&self, strategy: ProcessingStrategy) -> Result<Option<PerformanceMetrics>> {
        let metrics = self.performance_metrics.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock metrics: {}", e)))?;
        
        Ok(metrics.get(&strategy).cloned())
    }

    /// Clear the processor cache
    pub fn clear_cache(&self) -> Result<()> {
        let mut cache = self.processor_cache.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock cache: {}", e)))?;
        
        cache.clear();
        Ok(())
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> Result<(usize, usize)> {
        let cache = self.processor_cache.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock cache: {}", e)))?;
        
        // Return (current_size, capacity)
        Ok((cache.len(), cache.capacity()))
    }

    /// Create a cache key for processor caching
    fn create_cache_key(&self, strategy: ProcessingStrategy, config: &ProcessingConfig) -> String {
        format!("{:?}_{:?}_{}", strategy, config.strategy, config.max_threads)
    }

    /// Recommend strategy with performance considerations
    fn recommend_strategy_with_metrics(&self, input: &ProcessingInput) -> Result<ProcessingStrategy> {
        // Start with basic recommendation
        let base_strategy = strategy_recommend(input)?;
        
        // Consider performance metrics if available
        let metrics = self.performance_metrics.lock()
            .map_err(|e| ProcessingError::system_error(format!("Failed to lock metrics: {}", e)))?;

        if metrics.is_empty() {
            return Ok(base_strategy);
        }

        // Find the strategy with the best performance score
        let mut best_strategy = base_strategy;
        let mut best_score = 0.0;

        for (strategy, perf_metrics) in metrics.iter() {
            if perf_metrics.sample_count < 3 {
                continue; // Need at least 3 samples for reliable metrics
            }

            // Calculate performance score (higher is better)
            let score = perf_metrics.success_rate * perf_metrics.memory_efficiency / perf_metrics.avg_time_per_mb;
            
            if score > best_score {
                best_score = score;
                best_strategy = *strategy;
            }
        }

        Ok(best_strategy)
    }
}

impl Default for DefaultProcessorFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessorFactory for DefaultProcessorFactory {
    fn create_processor(
        &self,
        strategy: ProcessingStrategy,
        config: ProcessingConfig,
    ) -> Result<Box<dyn DataProcessor>> {
        // Check cache first
        let cache_key = self.create_cache_key(strategy, &config);
        
        {
            let cache = self.processor_cache.lock()
                .map_err(|e| ProcessingError::system_error(format!("Failed to lock cache: {}", e)))?;
            
            if let Some(_cached_processor) = cache.get(&cache_key) {
                // Note: We can't return the cached processor directly because of ownership issues
                // In a real implementation, you might use Arc<Mutex<dyn DataProcessor>> or similar
                // For now, we'll create a new processor each time
            }
        }

        // Create new processor
        let processor: Box<dyn DataProcessor> = match strategy {
            ProcessingStrategy::InMemory => {
                Box::new(InMemoryProcessor::new(config))
            }
            ProcessingStrategy::Chunked => {
                Box::new(ChunkedProcessor::new(config))
            }
            ProcessingStrategy::MemoryMapped => {
                Box::new(MemoryMappedProcessor::new(config))
            }
            ProcessingStrategy::Auto => {
                return Err(ProcessingError::invalid_configuration(
                    "Auto strategy must be resolved before creating processor".to_string()
                ));
            }
            ProcessingStrategy::Streaming => {
                return Err(Error::from(ProcessingError::unsupported_data_type(
                    "Streaming strategy is not yet implemented".to_string()
                )));
            }
        };

        Ok(processor)
    }

    fn create_pipeline(&self, config: ProcessingConfig) -> Result<Box<dyn ProcessingPipeline>> {
        Ok(Box::new(DefaultPipeline::new(config)))
    }

    fn supported_strategies(&self) -> Vec<ProcessingStrategy> {
        vec![
            ProcessingStrategy::InMemory,
            ProcessingStrategy::Chunked,
            ProcessingStrategy::MemoryMapped,
            ProcessingStrategy::Auto,
        ]
    }

    fn recommend_strategy(&self, input: &ProcessingInput) -> Result<ProcessingStrategy> {
        self.recommend_strategy_with_metrics(input)
    }
}

/// Create a default processor factory instance
pub fn create_factory() -> DefaultProcessorFactory {
    DefaultProcessorFactory::new()
}

/// Create a processor factory with performance optimization enabled
pub fn create_optimized_factory() -> DefaultProcessorFactory {
    let mut initial_metrics = HashMap::new();
    
    // Add some baseline performance metrics
    initial_metrics.insert(ProcessingStrategy::InMemory, PerformanceMetrics {
        avg_time_per_mb: 0.5,
        memory_efficiency: 0.8,
        success_rate: 0.95,
        sample_count: 10,
    });
    
    initial_metrics.insert(ProcessingStrategy::Chunked, PerformanceMetrics {
        avg_time_per_mb: 0.8,
        memory_efficiency: 0.95,
        success_rate: 0.98,
        sample_count: 10,
    });
    
    initial_metrics.insert(ProcessingStrategy::MemoryMapped, PerformanceMetrics {
        avg_time_per_mb: 1.2,
        memory_efficiency: 0.99,
        success_rate: 0.92,
        sample_count: 10,
    });

    DefaultProcessorFactory::with_metrics(initial_metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processing::traits::ProcessingInput;

    #[test]
    fn test_factory_creation() {
        let factory = DefaultProcessorFactory::new();
        assert_eq!(factory.cache_stats().unwrap(), (0, 0));
    }

    #[test]
    fn test_create_in_memory_processor() {
        let factory = DefaultProcessorFactory::new();
        let config = ProcessingConfig::default();
        let processor = factory.create_processor(ProcessingStrategy::InMemory, config);
        assert!(processor.is_ok());
    }

    #[test]
    fn test_create_chunked_processor() {
        let factory = DefaultProcessorFactory::new();
        let config = ProcessingConfig::default();
        let processor = factory.create_processor(ProcessingStrategy::Chunked, config);
        assert!(processor.is_ok());
    }

    #[test]
    fn test_create_mmap_processor() {
        let factory = DefaultProcessorFactory::new();
        let config = ProcessingConfig::default();
        let processor = factory.create_processor(ProcessingStrategy::MemoryMapped, config);
        assert!(processor.is_ok());
    }

    #[test]
    fn test_auto_strategy_error() {
        let factory = DefaultProcessorFactory::new();
        let config = ProcessingConfig::default();
        let result = factory.create_processor(ProcessingStrategy::Auto, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_strategy_not_implemented() {
        let factory = DefaultProcessorFactory::new();
        let config = ProcessingConfig::default();
        let result = factory.create_processor(ProcessingStrategy::Streaming, config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_pipeline() {
        let factory = DefaultProcessorFactory::new();
        let config = ProcessingConfig::default();
        let pipeline = factory.create_pipeline(config);
        assert!(pipeline.is_ok());
    }

    #[test]
    fn test_supported_strategies() {
        let factory = DefaultProcessorFactory::new();
        let strategies = factory.supported_strategies();
        assert!(strategies.contains(&ProcessingStrategy::InMemory));
        assert!(strategies.contains(&ProcessingStrategy::Chunked));
        assert!(strategies.contains(&ProcessingStrategy::MemoryMapped));
        assert!(strategies.contains(&ProcessingStrategy::Auto));
    }

    #[test]
    fn test_recommend_strategy() {
        let factory = DefaultProcessorFactory::new();
        let input = ProcessingInput::new(vec!["test.data".to_string()]);
        let strategy = factory.recommend_strategy(&input);
        assert!(strategy.is_ok());
    }

    #[test]
    fn test_metrics_update() {
        let factory = DefaultProcessorFactory::new();
        let result = factory.update_metrics(
            ProcessingStrategy::InMemory,
            10.0,  // processing_time
            100.0, // data_size_mb
            80.0,  // memory_used_mb
            true,  // success
        );
        assert!(result.is_ok());

        let metrics = factory.get_metrics(ProcessingStrategy::InMemory);
        assert!(metrics.is_ok());
        assert!(metrics.unwrap().is_some());
    }

    #[test]
    fn test_cache_operations() {
        let factory = DefaultProcessorFactory::new();
        
        // Initially empty
        assert_eq!(factory.cache_stats().unwrap(), (0, 0));
        
        // Clear cache (should not error even when empty)
        assert!(factory.clear_cache().is_ok());
    }

    #[test]
    fn test_optimized_factory() {
        let factory = create_optimized_factory();
        let metrics = factory.get_metrics(ProcessingStrategy::InMemory);
        assert!(metrics.is_ok());
        assert!(metrics.unwrap().is_some());
    }
}