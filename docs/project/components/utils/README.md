# Utils Module

The Utils module provides essential utility functions and helper components for the Rusty BLS Data Processing application. It offers a comprehensive set of tools for path manipulation, file operations, time handling, formatting, validation, and resource management.

## Overview

The Utils module is designed to:

1. Provide common utility functions used across all system components
2. Offer standardized path and file manipulation capabilities
3. Enable consistent time handling and formatting operations
4. Support comprehensive data validation and sanitization
5. Facilitate efficient resource management and monitoring

## Module Structure

```
src/utils/
├── mod.rs           # Module entry point and re-exports
├── path.rs          # Path manipulation utilities
├── file.rs          # File system operations and utilities
├── time.rs          # Time handling and formatting utilities
├── format.rs        # Data formatting and conversion utilities
├── validation.rs    # Data validation and sanitization utilities
└── resource.rs      # Resource management and monitoring utilities
```

## Key Components

### Path Utilities (`path.rs`)

The `path.rs` file provides comprehensive path manipulation capabilities:

- `PathUtils`: Core path manipulation and validation functions
- Path normalization and canonicalization
- Cross-platform path handling
- Path security validation and sanitization
- Temporary directory and file management

```rust
use crate::utils::PathUtils;

let path_utils = PathUtils::new();

// Normalize and validate paths
let normalized_path = path_utils.normalize_path("/path/to/../file.txt")?;
let is_safe = path_utils.is_safe_path(&normalized_path, "/allowed/base/path")?;

// Create temporary directories
let temp_dir = path_utils.create_temp_directory("rusty_processing")?;
```

### File Utilities (`file.rs`)

The `file.rs` file provides file system operations and utilities:

- `FileUtils`: File operations, permissions, and metadata handling
- Safe file reading and writing with validation
- File compression and decompression
- File integrity checking and verification
- Atomic file operations

```rust
use crate::utils::FileUtils;

let file_utils = FileUtils::new();

// Safe file operations
let content = file_utils.read_file_safe("/path/to/file.txt", 1024 * 1024)?; // 1MB limit
file_utils.write_file_atomic("/path/to/output.txt", &data)?;

// File integrity
let checksum = file_utils.calculate_checksum("/path/to/file.txt")?;
let is_valid = file_utils.verify_file_integrity("/path/to/file.txt", &expected_checksum)?;
```

### Time Utilities (`time.rs`)

The `time.rs` file provides time handling and formatting utilities:

- `TimeUtils`: Time parsing, formatting, and manipulation
- BLS-specific date format handling
- Time zone conversion and normalization
- Duration calculations and formatting
- Performance timing utilities

```rust
use crate::utils::TimeUtils;

let time_utils = TimeUtils::new();

// BLS date format handling
let date = time_utils.parse_bls_date("2023M12")?; // December 2023
let formatted = time_utils.format_bls_date(&date, "YYYY-MM")?;

// Performance timing
let timer = time_utils.start_timer();
// ... perform operations ...
let duration = timer.elapsed();
```

### Format Utilities (`format.rs`)

The `format.rs` file provides data formatting and conversion utilities:

- `FormatUtils`: Data formatting, conversion, and serialization
- Number formatting with locale support
- String manipulation and sanitization
- Data type conversion utilities
- Output format standardization

```rust
use crate::utils::FormatUtils;

let format_utils = FormatUtils::new();

// Number formatting
let formatted_number = format_utils.format_number(1234567.89, 2, Some("en_US"))?;
let parsed_number = format_utils.parse_number("1,234,567.89", Some("en_US"))?;

// String utilities
let sanitized = format_utils.sanitize_string(&input, &SanitizationRules::default())?;
let truncated = format_utils.truncate_string(&long_text, 100, "...")?;
```

### Validation Utilities (`validation.rs`)

The `validation.rs` file provides data validation and sanitization utilities:

- `ValidationUtils`: Comprehensive data validation framework
- Input sanitization and security validation
- Schema validation and data type checking
- Custom validation rule engine
- Error reporting and validation results

```rust
use crate::utils::ValidationUtils;

let validation_utils = ValidationUtils::new();

// Data validation
let survey_code_valid = validation_utils.validate_survey_code("AP")?;
let series_id_valid = validation_utils.validate_series_id("SERIES001")?;

// Custom validation rules
let rules = ValidationRules::new()
    .add_rule("length", |value| value.len() <= 100)
    .add_rule("format", |value| value.chars().all(|c| c.is_alphanumeric()));

let result = validation_utils.validate_with_rules(&input, &rules)?;
```

### Resource Utilities (`resource.rs`)

The `resource.rs` file provides resource management and monitoring utilities:

- `ResourceManager`: System resource monitoring and management
- Memory usage tracking and optimization
- CPU usage monitoring and throttling
- Disk space management and cleanup
- Resource limit enforcement

```rust
use crate::utils::ResourceManager;

let resource_manager = ResourceManager::new();

// Resource monitoring
let memory_usage = resource_manager.get_memory_usage()?;
let cpu_usage = resource_manager.get_cpu_usage()?;
let disk_space = resource_manager.get_disk_space("/data")?;

// Resource limits
resource_manager.set_memory_limit(1024 * 1024 * 1024)?; // 1GB
resource_manager.enforce_limits()?;
```

## Integration with Other Components

The Utils module integrates with other components by:

1. **Configuration System**: Providing path validation and file operations for configuration loading
2. **Data System**: Offering validation utilities for data integrity and format conversion
3. **Processing System**: Supporting resource management and performance monitoring
4. **Output System**: Enabling file operations and format utilities for output generation
5. **Error System**: Providing validation and sanitization for error handling
6. **Plugin System**: Offering resource management and security utilities for plugin execution

## Common Use Cases

### Path and File Operations

```rust
use crate::utils::{PathUtils, FileUtils};

// Safe file processing
fn process_survey_file(file_path: &str, base_dir: &str) -> Result<Survey> {
    let path_utils = PathUtils::new();
    let file_utils = FileUtils::new();
    
    // Validate and normalize path
    let safe_path = path_utils.validate_and_normalize(file_path, base_dir)?;
    
    // Read file with size limits
    let content = file_utils.read_file_safe(&safe_path, 100 * 1024 * 1024)?; // 100MB limit
    
    // Parse survey data
    let survey = parse_survey_data(&content)?;
    
    Ok(survey)
}
```

### Data Validation and Formatting

```rust
use crate::utils::{ValidationUtils, FormatUtils};

// Validate and format survey data
fn process_survey_data(raw_data: &RawSurveyData) -> Result<ProcessedSurveyData> {
    let validation_utils = ValidationUtils::new();
    let format_utils = FormatUtils::new();
    
    // Validate survey code
    validation_utils.validate_survey_code(&raw_data.code)?;
    
    // Format and validate series data
    let mut processed_series = Vec::new();
    for series in &raw_data.series {
        // Validate series ID
        validation_utils.validate_series_id(&series.id)?;
        
        // Format observations
        let formatted_observations: Result<Vec<_>> = series.observations
            .iter()
            .map(|obs| {
                let formatted_value = format_utils.format_number(obs.value, 2, None)?;
                Ok(FormattedObservation {
                    period: obs.period.clone(),
                    value: formatted_value,
                    flags: obs.flags.clone(),
                })
            })
            .collect();
        
        processed_series.push(ProcessedSeries {
            id: series.id.clone(),
            observations: formatted_observations?,
        });
    }
    
    Ok(ProcessedSurveyData {
        code: raw_data.code.clone(),
        series: processed_series,
    })
}
```

### Resource Management

```rust
use crate::utils::ResourceManager;

// Monitor and manage resources during processing
fn process_large_dataset(dataset: &LargeDataset) -> Result<ProcessedDataset> {
    let resource_manager = ResourceManager::new();
    
    // Set resource limits
    resource_manager.set_memory_limit(2 * 1024 * 1024 * 1024)?; // 2GB
    resource_manager.set_cpu_limit(80.0)?; // 80% CPU usage
    
    let mut processed_data = Vec::new();
    
    for chunk in dataset.chunks(1000) {
        // Check resource usage before processing each chunk
        let memory_usage = resource_manager.get_memory_usage()?;
        if memory_usage.percentage > 90.0 {
            // Trigger garbage collection or wait
            resource_manager.optimize_memory()?;
        }
        
        // Process chunk
        let processed_chunk = process_data_chunk(chunk)?;
        processed_data.extend(processed_chunk);
        
        // Enforce resource limits
        resource_manager.enforce_limits()?;
    }
    
    Ok(ProcessedDataset { data: processed_data })
}
```

## Performance Considerations

### Efficient String Operations

```rust
use crate::utils::FormatUtils;

// Use string builders for multiple concatenations
let format_utils = FormatUtils::new();
let mut builder = format_utils.create_string_builder();

for item in items {
    builder.append(&format!("Item: {}\n", item));
}

let result = builder.build();
```

### Memory-Efficient File Processing

```rust
use crate::utils::FileUtils;

// Stream large files instead of loading into memory
let file_utils = FileUtils::new();
let reader = file_utils.create_buffered_reader(file_path, 64 * 1024)?; // 64KB buffer

for line in reader.lines() {
    let line = line?;
    process_line(&line)?;
}
```

### Resource Monitoring

```rust
use crate::utils::ResourceManager;

// Continuous resource monitoring
let resource_manager = ResourceManager::new();
let monitor = resource_manager.start_monitoring(Duration::from_secs(5))?;

// Process data with monitoring
while let Some(data_chunk) = get_next_chunk() {
    let metrics = monitor.get_current_metrics();
    
    if metrics.memory_usage > 0.8 {
        // Reduce processing intensity
        process_chunk_conservatively(&data_chunk)?;
    } else {
        // Normal processing
        process_chunk_normally(&data_chunk)?;
    }
}
```

## Security Considerations

The Utils module implements several security measures:

- **Path Traversal Prevention**: All path operations validate against directory traversal attacks
- **Input Sanitization**: Comprehensive input validation and sanitization utilities
- **Resource Limits**: Enforcement of memory, CPU, and disk usage limits
- **File Permissions**: Proper file permission handling and validation
- **Data Validation**: Strict validation of all input data and parameters

## Testing and Validation

The Utils module includes comprehensive testing utilities:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::*;
    
    #[test]
    fn test_path_validation() {
        let path_utils = PathUtils::new();
        
        // Test safe paths
        assert!(path_utils.is_safe_path("/safe/path/file.txt", "/safe").unwrap());
        
        // Test path traversal prevention
        assert!(!path_utils.is_safe_path("/safe/../etc/passwd", "/safe").unwrap());
    }
    
    #[test]
    fn test_file_operations() {
        let file_utils = FileUtils::new();
        let temp_file = create_temp_file_with_content("test content");
        
        let content = file_utils.read_file_safe(&temp_file, 1024).unwrap();
        assert_eq!(content, "test content");
    }
}
```

For more detailed information, see:

- [Best Practices](best_practices.md)
- [Design Patterns](design_patterns.md)
- [Integration Guidelines](integration.md)
- [Tasks and Roadmap](tasks.md)
- [Test Specifications](test_specifications.md)
- [Architecture Validation](validation.md)