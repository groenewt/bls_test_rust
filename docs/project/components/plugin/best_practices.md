# Plugin Module: Best Practices

This document outlines best practices for developing, deploying, and using plugins in the Rusty BLS Data Processing system.

## Plugin Development

### 1. Follow the Plugin Interface Contract

Always implement the required plugin traits correctly:

✅ **Good**:
```rust
use crate::plugin::{Plugin, SurveyPlugin, PluginMetadata};
use crate::error::Result;

pub struct MySurveyPlugin {
    initialized: bool,
}

impl Plugin for MySurveyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "My Survey Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "Processes MY survey data".to_string(),
            author: "Developer Name".to_string(),
            capabilities: vec!["survey_processing".to_string()],
            dependencies: vec![],
        }
    }
    
    fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }
        
        // Perform initialization
        self.initialized = true;
        log::info!("My Survey Plugin initialized successfully");
        Ok(())
    }
    
    fn shutdown(&mut self) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }
        
        // Perform cleanup
        self.initialized = false;
        log::info!("My Survey Plugin shut down successfully");
        Ok(())
    }
}
```

❌ **Bad**:
```rust
impl Plugin for MySurveyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::default() // Incomplete metadata
    }
    
    fn initialize(&mut self) -> Result<()> {
        // No initialization logic
        Ok(())
    }
    
    fn shutdown(&mut self) -> Result<()> {
        // No cleanup logic
        Ok(())
    }
}
```

### 2. Implement Proper Error Handling

Use specific error types and provide detailed context:

✅ **Good**:
```rust
impl SurveyPlugin for MySurveyPlugin {
    fn process_survey(&self, survey: &mut Survey) -> Result<()> {
        if !self.supports_survey(&survey.code) {
            return Err(PluginError::UnsupportedSurvey {
                plugin_name: "MySurveyPlugin".to_string(),
                survey_code: survey.code.clone(),
            }.into());
        }
        
        for series in &mut survey.series {
            self.process_series(series)
                .with_context(format!(
                    "Failed to process series {} in survey {}",
                    series.id, survey.code
                ))?;
        }
        
        Ok(())
    }
}
```

❌ **Bad**:
```rust
impl SurveyPlugin for MySurveyPlugin {
    fn process_survey(&self, survey: &mut Survey) -> Result<()> {
        for series in &mut survey.series {
            self.process_series(series)?; // No context
        }
        Ok(())
    }
}
```

### 3. Use Configuration-Driven Behavior

Make plugins configurable rather than hardcoded:

✅ **Good**:
```rust
#[derive(Deserialize)]
pub struct MySurveyPluginConfig {
    pub validation_level: ValidationLevel,
    pub custom_transformations: Vec<String>,
    pub output_precision: u8,
}

impl Default for MySurveyPluginConfig {
    fn default() -> Self {
        Self {
            validation_level: ValidationLevel::Standard,
            custom_transformations: vec![],
            output_precision: 2,
        }
    }
}

pub struct MySurveyPlugin {
    config: MySurveyPluginConfig,
}

impl MySurveyPlugin {
    pub fn new(config: MySurveyPluginConfig) -> Self {
        Self { config }
    }
    
    fn process_series(&self, series: &mut Series) -> Result<()> {
        // Use configuration to drive behavior
        match self.config.validation_level {
            ValidationLevel::Strict => self.strict_validation(series)?,
            ValidationLevel::Standard => self.standard_validation(series)?,
            ValidationLevel::Lenient => self.lenient_validation(series)?,
        }
        
        for transformation in &self.config.custom_transformations {
            self.apply_transformation(series, transformation)?;
        }
        
        Ok(())
    }
}
```

❌ **Bad**:
```rust
impl MySurveyPlugin {
    fn process_series(&self, series: &mut Series) -> Result<()> {
        // Hardcoded behavior
        self.strict_validation(series)?;
        self.apply_transformation(series, "normalize")?;
        Ok(())
    }
}
```

## Plugin Architecture

### 1. Design for Modularity

Create focused plugins with single responsibilities:

✅ **Good**:
```rust
// Separate plugins for different concerns
pub struct DataValidationPlugin;
pub struct DataTransformationPlugin;
pub struct DataOutputPlugin;

impl SurveyPlugin for DataValidationPlugin {
    fn process_survey(&self, survey: &mut Survey) -> Result<()> {
        // Only validation logic
        self.validate_survey_structure(survey)?;
        self.validate_data_integrity(survey)?;
        Ok(())
    }
}
```

❌ **Bad**:
```rust
// Monolithic plugin doing everything
pub struct AllInOneSurveyPlugin;

impl SurveyPlugin for AllInOneSurveyPlugin {
    fn process_survey(&self, survey: &mut Survey) -> Result<()> {
        // Validation, transformation, and output all mixed together
        self.validate_and_transform_and_output(survey)?;
        Ok(())
    }
}
```

### 2. Implement Plugin Dependencies Correctly

Declare and manage plugin dependencies properly:

✅ **Good**:
```rust
impl Plugin for AdvancedProcessingPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Advanced Processing Plugin".to_string(),
            version: "1.0.0".to_string(),
            dependencies: vec![
                PluginDependency {
                    name: "data_validation_plugin".to_string(),
                    version_requirement: ">=1.0.0".to_string(),
                    required: true,
                },
                PluginDependency {
                    name: "statistics_plugin".to_string(),
                    version_requirement: ">=2.1.0".to_string(),
                    required: false,
                },
            ],
            // ... other metadata
        }
    }
}
```

❌ **Bad**:
```rust
impl Plugin for AdvancedProcessingPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            dependencies: vec![], // No dependencies declared
            // ... other metadata
        }
    }
    
    fn initialize(&mut self) -> Result<()> {
        // Assumes other plugins are available without declaring dependency
        let validator = get_plugin("data_validation_plugin")?;
        Ok(())
    }
}
```

### 3. Use Plugin Communication Patterns

Implement proper communication between plugins:

✅ **Good**:
```rust
use crate::plugin::PluginMessage;

pub struct CoordinatorPlugin {
    message_bus: Arc<Mutex<MessageBus>>,
}

impl Plugin for CoordinatorPlugin {
    fn initialize(&mut self) -> Result<()> {
        // Subscribe to relevant messages
        self.message_bus.lock().unwrap()
            .subscribe("data_processed", self.handle_data_processed)?;
        Ok(())
    }
}

impl CoordinatorPlugin {
    fn handle_data_processed(&self, message: PluginMessage) -> Result<()> {
        // React to messages from other plugins
        match message.payload {
            MessagePayload::DataProcessed { survey_code, .. } => {
                self.trigger_next_stage(&survey_code)?;
            }
            _ => {}
        }
        Ok(())
    }
}
```

❌ **Bad**:
```rust
// Direct coupling between plugins
impl ProcessingPlugin {
    fn process_data(&self, data: &Data) -> Result<()> {
        // Directly calling another plugin - tight coupling
        let output_plugin = unsafe { get_global_output_plugin() };
        output_plugin.write_data(data)?;
        Ok(())
    }
}
```

## Performance Optimization

### 1. Minimize Plugin Overhead

Design plugins to have minimal performance impact:

✅ **Good**:
```rust
pub struct EfficientPlugin {
    // Pre-allocate buffers
    buffer: RefCell<Vec<u8>>,
    // Cache expensive computations
    computation_cache: RefCell<HashMap<String, ComputationResult>>,
}

impl EfficientPlugin {
    fn process_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Reuse buffer to avoid allocations
        let mut buffer = self.buffer.borrow_mut();
        buffer.clear();
        buffer.reserve(data.len() * 2);
        
        // Use cache for expensive operations
        let cache_key = self.compute_cache_key(data);
        if let Some(cached_result) = self.computation_cache.borrow().get(&cache_key) {
            return Ok(cached_result.data.clone());
        }
        
        // Perform computation and cache result
        let result = self.expensive_computation(data)?;
        self.computation_cache.borrow_mut().insert(cache_key, result.clone());
        
        Ok(result.data)
    }
}
```

❌ **Bad**:
```rust
impl InefficientPlugin {
    fn process_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // New allocation every time
        let mut result = Vec::new();
        
        // Expensive computation without caching
        for chunk in data.chunks(1024) {
            let processed = self.expensive_computation(chunk)?;
            result.extend(processed);
        }
        
        Ok(result)
    }
}
```

### 2. Use Async Processing When Appropriate

Implement async processing for I/O-bound operations:

✅ **Good**:
```rust
#[async_trait]
pub trait AsyncSurveyPlugin: Plugin {
    async fn process_survey_async(&self, survey: &mut Survey) -> Result<()>;
}

#[async_trait]
impl AsyncSurveyPlugin for NetworkOutputPlugin {
    async fn process_survey_async(&self, survey: &mut Survey) -> Result<()> {
        let tasks: Vec<_> = survey.series.iter()
            .map(|series| self.upload_series_async(series))
            .collect();
        
        // Process all series concurrently
        let results = futures::future::join_all(tasks).await;
        
        for result in results {
            result?;
        }
        
        Ok(())
    }
}
```

❌ **Bad**:
```rust
impl NetworkOutputPlugin {
    fn process_survey(&self, survey: &mut Survey) -> Result<()> {
        // Sequential processing of network operations
        for series in &survey.series {
            self.upload_series_sync(series)?; // Blocking I/O
        }
        Ok(())
    }
}
```

### 3. Implement Resource Management

Properly manage resources in plugins:

✅ **Good**:
```rust
pub struct ResourceManagedPlugin {
    file_handles: RefCell<Vec<File>>,
    memory_pool: Arc<MemoryPool>,
    max_memory_usage: usize,
}

impl Plugin for ResourceManagedPlugin {
    fn initialize(&mut self) -> Result<()> {
        // Set resource limits
        self.memory_pool.set_limit(self.max_memory_usage);
        Ok(())
    }
    
    fn shutdown(&mut self) -> Result<()> {
        // Clean up resources
        for file in self.file_handles.borrow_mut().drain(..) {
            drop(file);
        }
        self.memory_pool.clear();
        Ok(())
    }
}

impl Drop for ResourceManagedPlugin {
    fn drop(&mut self) {
        // Ensure cleanup even if shutdown wasn't called
        let _ = self.shutdown();
    }
}
```

❌ **Bad**:
```rust
pub struct ResourceLeakyPlugin {
    files: RefCell<Vec<File>>,
}

impl Plugin for ResourceLeakyPlugin {
    fn shutdown(&mut self) -> Result<()> {
        // No resource cleanup
        Ok(())
    }
}
// No Drop implementation - resources may leak
```

## Security Best Practices

### 1. Validate All Inputs

Always validate data received from the plugin system:

✅ **Good**:
```rust
impl SurveyPlugin for SecurePlugin {
    fn process_survey(&self, survey: &mut Survey) -> Result<()> {
        // Validate survey structure
        if survey.code.is_empty() || survey.code.len() > 10 {
            return Err(PluginError::InvalidInput {
                message: "Survey code must be 1-10 characters".to_string(),
            }.into());
        }
        
        // Validate series data
        for series in &survey.series {
            if series.observations.len() > MAX_OBSERVATIONS {
                return Err(PluginError::InvalidInput {
                    message: format!(
                        "Series {} has too many observations: {}",
                        series.id, series.observations.len()
                    ),
                }.into());
            }
        }
        
        Ok(())
    }
}
```

❌ **Bad**:
```rust
impl InsecurePlugin {
    fn process_survey(&self, survey: &mut Survey) -> Result<()> {
        // No input validation
        self.process_data(&survey.code)?;
        Ok(())
    }
}
```

### 2. Implement Capability-Based Security

Declare and respect plugin capabilities:

✅ **Good**:
```rust
impl Plugin for FileOutputPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            capabilities: vec![
                "file_write".to_string(),
                "directory_create".to_string(),
            ],
            // ... other metadata
        }
    }
}

impl FileOutputPlugin {
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        // Check capability before performing operation
        if !self.has_capability("file_write") {
            return Err(PluginError::InsufficientCapabilities {
                required: "file_write".to_string(),
            }.into());
        }
        
        // Validate path is within allowed directories
        self.validate_file_path(path)?;
        
        std::fs::write(path, data)?;
        Ok(())
    }
}
```

❌ **Bad**:
```rust
impl FileOutputPlugin {
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<()> {
        // No capability checking or path validation
        std::fs::write(path, data)?;
        Ok(())
    }
}
```

### 3. Sanitize Error Messages

Prevent information leakage through error messages:

✅ **Good**:
```rust
impl DatabasePlugin {
    fn connect_to_database(&self, connection_string: &str) -> Result<Connection> {
        match Database::connect(connection_string) {
            Ok(conn) => Ok(conn),
            Err(e) => {
                // Log detailed error internally
                log::error!("Database connection failed: {}", e);
                
                // Return sanitized error to caller
                Err(PluginError::DatabaseConnectionFailed {
                    message: "Failed to connect to database".to_string(),
                }.into())
            }
        }
    }
}
```

❌ **Bad**:
```rust
impl DatabasePlugin {
    fn connect_to_database(&self, connection_string: &str) -> Result<Connection> {
        Database::connect(connection_string)
            .map_err(|e| PluginError::DatabaseError {
                message: format!("Connection failed: {}", e), // May leak sensitive info
            }.into())
    }
}
```

## Testing

### 1. Write Comprehensive Plugin Tests

Test all plugin functionality thoroughly:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    
    #[test]
    fn test_plugin_initialization() {
        let mut plugin = MySurveyPlugin::new(Default::default());
        
        assert!(plugin.initialize().is_ok());
        assert!(plugin.initialized);
        
        // Test idempotency
        assert!(plugin.initialize().is_ok());
    }
    
    #[test]
    fn test_plugin_shutdown() {
        let mut plugin = MySurveyPlugin::new(Default::default());
        plugin.initialize().unwrap();
        
        assert!(plugin.shutdown().is_ok());
        assert!(!plugin.initialized);
        
        // Test idempotency
        assert!(plugin.shutdown().is_ok());
    }
    
    #[test]
    fn test_survey_processing() {
        let plugin = MySurveyPlugin::new(Default::default());
        let mut survey = create_test_survey("MY");
        
        let result = plugin.process_survey(&mut survey);
        assert!(result.is_ok());
        
        // Verify expected transformations
        assert_eq!(survey.series.len(), 3);
        assert!(survey.metadata.contains_key("processed_by"));
    }
    
    #[test]
    fn test_unsupported_survey() {
        let plugin = MySurveyPlugin::new(Default::default());
        let mut survey = create_test_survey("UNSUPPORTED");
        
        let result = plugin.process_survey(&mut survey);
        assert!(result.is_err());
        
        match result.unwrap_err().downcast_ref::<PluginError>() {
            Some(PluginError::UnsupportedSurvey { .. }) => {},
            _ => panic!("Expected UnsupportedSurvey error"),
        }
    }
}
```

### 2. Test Plugin Integration

Test how plugins work together:

```rust
#[test]
fn test_plugin_chain() {
    let mut registry = PluginRegistry::new();
    
    // Register plugins in dependency order
    registry.register(Box::new(ValidationPlugin::new()))?;
    registry.register(Box::new(TransformationPlugin::new()))?;
    registry.register(Box::new(OutputPlugin::new()))?;
    
    let mut survey = create_test_survey("TEST");
    
    // Process through plugin chain
    let result = registry.process_survey(&mut survey);
    assert!(result.is_ok());
    
    // Verify each plugin's effects
    assert!(survey.metadata.contains_key("validated"));
    assert!(survey.metadata.contains_key("transformed"));
    assert!(survey.metadata.contains_key("output_generated"));
}
```

### 3. Test Error Conditions

Test plugin behavior under error conditions:

```rust
#[test]
fn test_plugin_error_handling() {
    let plugin = MySurveyPlugin::new(Default::default());
    let mut invalid_survey = Survey::default(); // Invalid survey
    
    let result = plugin.process_survey(&mut invalid_survey);
    assert!(result.is_err());
    
    // Verify error contains useful information
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Survey code"));
}

#[test]
fn test_plugin_resource_exhaustion() {
    let config = MySurveyPluginConfig {
        max_memory_usage: 1024, // Very low limit
        ..Default::default()
    };
    let plugin = MySurveyPlugin::new(config);
    
    let mut large_survey = create_large_test_survey(10000);
    
    let result = plugin.process_survey(&mut large_survey);
    assert!(result.is_err());
    
    // Should handle resource exhaustion gracefully
    match result.unwrap_err().downcast_ref::<PluginError>() {
        Some(PluginError::ResourceExhausted { .. }) => {},
        _ => panic!("Expected ResourceExhausted error"),
    }
}
```

## Deployment and Configuration

### 1. Use Versioned Plugin Deployment

Deploy plugins with proper versioning:

```yaml
# Plugin configuration
plugins:
  survey_plugins:
    - name: "my_survey_plugin"
      version: "1.2.3"
      path: "plugins/v1.2.3/libmy_survey.so"
      checksum: "sha256:abc123..."
      config:
        validation_level: "strict"
        
  # Fallback to older version if needed
  fallback_plugins:
    - name: "my_survey_plugin"
      version: "1.1.0"
      path: "plugins/v1.1.0/libmy_survey.so"
      checksum: "sha256:def456..."
```

### 2. Implement Plugin Health Checks

Monitor plugin health and performance:

```rust
impl Plugin for MonitoredPlugin {
    fn health_check(&self) -> Result<PluginHealth> {
        let mut health = PluginHealth::new();
        
        // Check memory usage
        let memory_usage = self.get_memory_usage();
        health.add_metric("memory_usage_mb", memory_usage as f64 / 1024.0 / 1024.0);
        
        // Check processing rate
        let processing_rate = self.get_processing_rate();
        health.add_metric("processing_rate_per_sec", processing_rate);
        
        // Check error rate
        let error_rate = self.get_error_rate();
        health.add_metric("error_rate_percent", error_rate * 100.0);
        
        if error_rate > 0.05 { // 5% error rate threshold
            health.set_status(PluginStatus::Degraded);
            health.add_warning("High error rate detected");
        }
        
        Ok(health)
    }
}
```

This document provides comprehensive best practices for developing, deploying, and maintaining plugins in the Rusty BLS Data Processing system.