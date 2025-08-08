# Configuration Module: Best Practices

This document outlines best practices for using the Configuration module.

## Configuration Design

### 1. Use Structured Configuration
```rust
// Good: Structured configuration
#[derive(Deserialize, Validate)]
pub struct ProcessingConfig {
    #[validate(range(min = 1, max = 32))]
    pub max_threads: u32,
    
    #[validate(range(min = 1000))]
    pub chunk_size: usize,
    
    pub strategy: ProcessingStrategy,
}

// Avoid: Flat configuration with magic values
```

### 2. Validate Early and Often
```rust
pub fn load_config(path: &Path) -> Result<Config> {
    let config = Config::load_from_file(path)?;
    
    // Validate immediately after loading
    config.validate()?;
    
    Ok(config)
}
```

### 3. Use Environment-Specific Overrides
```rust
pub fn load_config_with_env() -> Result<Config> {
    let mut config = Config::load_from_file("config.yml")?;
    
    // Apply environment-specific overrides
    if let Ok(env_overrides) = Config::from_env() {
        config.merge(env_overrides);
    }
    
    config.validate()?;
    Ok(config)
}
```

## Error Handling

### 1. Provide Clear Error Messages
```rust
impl Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingFile { path } => {
                write!(f, "Configuration file not found: {}", path.display())
            }
            ConfigError::ValidationError { field, message } => {
                write!(f, "Invalid configuration for '{}': {}", field, message)
            }
        }
    }
}
```

### 2. Use Context for Better Debugging
```rust
pub fn load_survey_config(survey_code: &str) -> Result<SurveyConfig> {
    let path = format!("config/surveys/{}.yml", survey_code);
    
    Config::load_from_file(&path)
        .with_context(|| format!("Failed to load configuration for survey '{}'", survey_code))
}
```

## Performance

### 1. Cache Frequently Used Configurations
```rust
pub struct ConfigCache {
    cache: HashMap<String, Arc<Config>>,
}

impl ConfigCache {
    pub fn get_or_load(&mut self, key: &str) -> Result<Arc<Config>> {
        if let Some(config) = self.cache.get(key) {
            return Ok(Arc::clone(config));
        }
        
        let config = Arc::new(Config::load(key)?);
        self.cache.insert(key.to_string(), Arc::clone(&config));
        Ok(config)
    }
}
```

### 2. Use Lazy Loading for Large Configurations
```rust
pub struct LazyConfig {
    loader: Box<dyn Fn() -> Result<Config>>,
    cached: OnceCell<Config>,
}

impl LazyConfig {
    pub fn get(&self) -> Result<&Config> {
        self.cached.get_or_try_init(&self.loader)
    }
}
```

## Security

### 1. Sanitize Sensitive Information
```rust
impl Config {
    pub fn sanitize(&self) -> Config {
        let mut sanitized = self.clone();
        sanitized.database.password = "[REDACTED]".to_string();
        sanitized.api_keys.clear();
        sanitized
    }
}
```

### 2. Validate Input Sources
```rust
pub fn load_config_safely(path: &Path) -> Result<Config> {
    // Validate path is within allowed directories
    if !path.starts_with("config/") {
        return Err(ConfigError::InvalidPath);
    }
    
    Config::load_from_file(path)
}
```

## Testing

### 1. Use Test-Specific Configurations
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_config() -> Config {
        Config {
            survey: SurveyConfig::new("TEST"),
            processing: ProcessingConfig::test_defaults(),
            output: OutputConfig::test_defaults(),
        }
    }
    
    #[test]
    fn test_processing_with_config() {
        let config = test_config();
        let processor = DataProcessor::from_config(&config.processing).unwrap();
        // Test with known configuration
    }
}
```

### 2. Test Configuration Validation
```rust
#[test]
fn test_invalid_configurations() {
    let mut config = Config::default();
    
    // Test invalid thread count
    config.processing.max_threads = 0;
    assert!(config.validate().is_err());
    
    // Test invalid chunk size
    config.processing.chunk_size = 0;
    assert!(config.validate().is_err());
}
```