# Processing Module: Best Practices

This document outlines best practices for using the Processing module.

## Strategy Selection

### 1. Choose Strategy Based on Data Size
```rust
pub fn select_processing_strategy(data_size: u64, memory_available: u64) -> Box<dyn ProcessingStrategy> {
    match (data_size, memory_available) {
        // Small datasets: use in-memory for speed
        (0..=100_000_000, _) => Box::new(InMemoryStrategy::new()),
        
        // Medium datasets: use chunked processing
        (100_000_001..=1_000_000_000, mem) if mem > data_size * 2 => {
            Box::new(ChunkedStrategy::new(10_000))
        }
        
        // Large datasets: use memory-mapped processing
        _ => Box::new(MmapStrategy::new()),
    }
}
```

### 2. Configure Strategy Parameters
```rust
impl InMemoryStrategy {
    pub fn with_optimal_config(data_size: usize) -> Self {
        let thread_count = std::cmp::min(
            num_cpus::get(),
            (data_size / 10_000).max(1)
        );
        
        Self::new().with_threads(thread_count)
    }
}
```

## Pipeline Design

### 1. Keep Stages Focused and Composable
```rust
// Good: Single responsibility stages
pub struct DataValidationStage {
    rules: Vec<ValidationRule>,
}

pub struct DataTransformationStage {
    transformers: Vec<Box<dyn DataTransformer>>,
}

// Avoid: Monolithic stages that do everything
```

### 2. Use Builder Pattern for Complex Pipelines
```rust
pub fn create_survey_pipeline(survey_code: &str) -> Result<ProcessingPipeline> {
    PipelineBuilder::new()
        .add_loader(LoaderType::BLS)
        .add_validator(ValidatorType::Survey(survey_code.to_string()))
        .add_transformer(TransformerType::StandardBLS)
        .add_writer(WriterType::CSV)
        .build()
}
```

## Performance Optimization

### 1. Use Parallel Processing Appropriately
```rust
pub fn process_with_parallelism(data: Vec<Observation>) -> Result<Vec<ProcessedData>> {
    use rayon::prelude::*;
    
    // Only use parallel processing for large datasets
    if data.len() > 10_000 {
        data.into_par_iter()
            .map(|obs| process_observation(obs))
            .collect()
    } else {
        data.into_iter()
            .map(|obs| process_observation(obs))
            .collect()
    }
}
```

### 2. Implement Chunked Processing for Memory Efficiency
```rust
pub fn process_large_dataset(data: Vec<Observation>) -> Result<Vec<ProcessedData>> {
    const CHUNK_SIZE: usize = 10_000;
    let mut results = Vec::with_capacity(data.len());
    
    for chunk in data.chunks(CHUNK_SIZE) {
        let processed_chunk = process_chunk(chunk)?;
        results.extend(processed_chunk);
        
        // Optional: yield control to allow other tasks
        if results.len() % (CHUNK_SIZE * 10) == 0 {
            tokio::task::yield_now().await;
        }
    }
    
    Ok(results)
}
```

### 3. Monitor Memory Usage
```rust
pub fn process_with_memory_monitoring(data: Vec<Observation>) -> Result<Vec<ProcessedData>> {
    let initial_memory = get_memory_usage();
    let max_memory_increase = 1_000_000_000; // 1GB
    
    let mut results = Vec::new();
    
    for (i, observation) in data.into_iter().enumerate() {
        let processed = process_observation(observation)?;
        results.push(processed);
        
        // Check memory usage periodically
        if i % 10_000 == 0 {
            let current_memory = get_memory_usage();
            if current_memory - initial_memory > max_memory_increase {
                return Err(ProcessingError::MemoryLimitExceeded);
            }
        }
    }
    
    Ok(results)
}
```

## Error Handling

### 1. Implement Graceful Error Recovery
```rust
pub fn process_with_recovery(data: Vec<Observation>) -> ProcessingResult {
    let mut successful = Vec::new();
    let mut failed = Vec::new();
    
    for observation in data {
        match process_observation(observation.clone()) {
            Ok(processed) => successful.push(processed),
            Err(e) => {
                log::warn!("Failed to process observation {}: {}", observation.id, e);
                failed.push(ProcessingFailure {
                    observation,
                    error: e,
                });
            }
        }
    }
    
    ProcessingResult {
        successful,
        failed,
        total_processed: successful.len() + failed.len(),
    }
}
```

### 2. Provide Detailed Error Context
```rust
pub fn process_survey_data(survey_code: &str, data: Vec<Observation>) -> Result<Vec<ProcessedData>> {
    let processor = create_survey_processor(survey_code)
        .with_context(|| format!("Failed to create processor for survey '{}'", survey_code))?;
    
    processor.process(data)
        .with_context(|| format!("Failed to process data for survey '{}'", survey_code))
}
```

## Testing

### 1. Test Different Data Sizes and Scenarios
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_small_dataset_processing() {
        let data = generate_test_data(100);
        let result = InMemoryStrategy::new().process(data).unwrap();
        assert_eq!(result.len(), 100);
    }
    
    #[test]
    fn test_large_dataset_processing() {
        let data = generate_test_data(1_000_000);
        let result = ChunkedStrategy::new(10_000).process(data).unwrap();
        assert_eq!(result.len(), 1_000_000);
    }
    
    #[test]
    fn test_memory_constrained_processing() {
        let data = generate_large_test_data();
        let result = MmapStrategy::new().process(data).unwrap();
        assert!(!result.is_empty());
    }
}
```

### 2. Test Error Scenarios
```rust
#[test]
fn test_invalid_data_handling() {
    let data = vec![
        valid_observation(),
        invalid_observation(),
        valid_observation(),
    ];
    
    let result = process_with_recovery(data);
    assert_eq!(result.successful.len(), 2);
    assert_eq!(result.failed.len(), 1);
}
```

## Configuration

### 1. Use Environment-Specific Settings
```rust
pub fn create_processing_config(environment: &str) -> ProcessingConfig {
    match environment {
        "development" => ProcessingConfig {
            strategy: StrategyType::InMemory,
            max_threads: 2,
            chunk_size: 1_000,
            enable_logging: true,
        },
        "production" => ProcessingConfig {
            strategy: StrategyType::Chunked,
            max_threads: num_cpus::get(),
            chunk_size: 10_000,
            enable_logging: false,
        },
        _ => ProcessingConfig::default(),
    }
}
```

### 2. Validate Configuration at Startup
```rust
pub fn validate_processing_config(config: &ProcessingConfig) -> Result<()> {
    if config.max_threads == 0 {
        return Err(ConfigError::InvalidThreadCount);
    }
    
    if config.chunk_size == 0 {
        return Err(ConfigError::InvalidChunkSize);
    }
    
    // Validate strategy-specific settings
    match config.strategy {
        StrategyType::Chunked if config.chunk_size > 100_000 => {
            return Err(ConfigError::ChunkSizeTooLarge);
        }
        _ => {}
    }
    
    Ok(())
}
```