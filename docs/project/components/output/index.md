# Output Layer Documentation

The Output Layer is a comprehensive enterprise-level component of the Rusty BLS Data Processing system that handles the generation and formatting of processed data into various output formats. This layer is designed for high performance, scalability, and flexibility.

## Architecture Overview

The Output Layer follows a modular, trait-based architecture that enables:

- **Format Flexibility**: Support for multiple output formats (CSV, Parquet, JSON)
- **Performance Optimization**: Different strategies for different data sizes and requirements
- **Enterprise Features**: Compression, partitioning, validation, and monitoring
- **Extensibility**: Easy addition of new output formats through the plugin system

## Core Components

### 1. Output Traits

The output system is built around several key traits:

#### OutputGenerator
The main trait that all output generators must implement:

```rust
#[async_trait]
pub trait OutputGenerator: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn supported_formats(&self) -> Vec<String>;
    fn can_handle_format(&self, format: &str) -> bool;
    async fn generate(&mut self, data: ProcessedData, config: OutputConfig) -> Result<OutputResult>;
    fn validate_config(&self, config: &OutputConfig) -> Result<()>;
    fn stats(&self) -> OutputStats;
    fn reset_stats(&mut self);
}
```

#### FormatWriter
Specialized trait for format-specific writing operations:

```rust
#[async_trait]
pub trait FormatWriter: Send + Sync {
    fn format_name(&self) -> &str;
    fn file_extension(&self) -> &str;
    async fn write_series(&mut self, series: &[Series], path: &Path, config: &OutputConfig) -> Result<OutputResult>;
    async fn write_observations(&mut self, observations: &[Observation], path: &Path, config: &OutputConfig) -> Result<OutputResult>;
    async fn write_lookups(&mut self, lookups: &[Lookup], path: &Path, config: &OutputConfig) -> Result<OutputResult>;
    async fn write_survey(&mut self, survey: &Survey, path: &Path, config: &OutputConfig) -> Result<OutputResult>;
    async fn write_mixed(&mut self, data: ProcessedData, path: &Path, config: &OutputConfig) -> Result<OutputResult>;
    fn validate_format_config(&self, config: &OutputConfig) -> Result<()>;
    fn default_format_options(&self) -> HashMap<String, String>;
}
```

### 2. Configuration System

#### OutputConfig
Comprehensive configuration for output generation:

```rust
pub struct OutputConfig {
    pub output_path: PathBuf,
    pub format: String,
    pub compression: CompressionConfig,
    pub partitioning: PartitioningConfig,
    pub format_options: HashMap<String, String>,
    pub validation_enabled: bool,
    pub overwrite_existing: bool,
    pub create_directories: bool,
    pub file_permissions: Option<u32>,
    pub buffer_size: usize,
    pub max_file_size: Option<u64>,
}
```

#### CompressionConfig
Configuration for data compression:

```rust
pub struct CompressionConfig {
    pub enabled: bool,
    pub algorithm: CompressionAlgorithm,
    pub level: u8,
    pub dictionary: Option<PathBuf>,
}

pub enum CompressionAlgorithm {
    None,
    Gzip,
    Zstd,
    Lz4,
    Snappy,
}
```

#### PartitioningConfig
Configuration for data partitioning:

```rust
pub struct PartitioningConfig {
    pub enabled: bool,
    pub strategy: PartitionStrategy,
    pub partition_size: u64,
    pub naming_strategy: PartitionNamingStrategy,
}

pub enum PartitionStrategy {
    None,
    BySize,
    ByCount,
    ByTime,
    ByField(String),
}
```

### 3. Format Implementations

#### CSV Format
High-performance CSV output with customizable options:

- Custom delimiters and quote characters
- Header row configuration
- Null value handling
- Character encoding options
- Streaming for large datasets

#### Parquet Format
Columnar storage format optimized for analytics:

- Schema inference and validation
- Compression algorithms (Snappy, Gzip, LZ4, Zstd)
- Row group size optimization
- Metadata preservation
- Predicate pushdown support

#### JSON Format
Flexible JSON output with various formatting options:

- Pretty printing and compact formats
- Custom date/time formatting
- Null value handling
- Streaming JSON for large datasets
- JSON Lines format support

### 4. Registry and Factory

#### OutputRegistry
Manages registered output generators:

```rust
#[async_trait]
pub trait OutputRegistry: Send + Sync {
    async fn register_generator(&mut self, name: String, generator: Box<dyn OutputGenerator>) -> Result<()>;
    async fn unregister_generator(&mut self, name: &str) -> Result<()>;
    fn get_generator(&self, name: &str) -> Result<&dyn OutputGenerator>;
    fn get_generator_mut(&mut self, name: &str) -> Result<&mut dyn OutputGenerator>;
    fn list_generators(&self) -> Vec<String>;
    fn has_generator(&self, name: &str) -> bool;
    fn get_generator_for_format(&self, format: &str) -> Result<&dyn OutputGenerator>;
    fn clear(&mut self);
}
```

#### OutputFactory
Creates output generators and writers:

```rust
#[async_trait]
pub trait OutputFactory: Send + Sync {
    async fn create_generator(&self, format: &str, config: OutputConfig) -> Result<Box<dyn OutputGenerator>>;
    async fn create_writer(&self, format: &str) -> Result<Box<dyn FormatWriter>>;
    fn supported_formats(&self) -> Vec<String>;
    fn supports_format(&self, format: &str) -> bool;
    fn default_config_for_format(&self, format: &str) -> Result<OutputConfig>;
}
```

## Usage Examples

### Basic Usage

```rust
use rusty::output::{OutputConfig, DefaultOutputFactory};
use rusty::processing::ProcessedData;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create output factory
    let factory = DefaultOutputFactory::new();
    
    // Configure output
    let config = OutputConfig {
        output_path: PathBuf::from("output/processed_data.csv"),
        format: "csv".to_string(),
        compression: CompressionConfig::default(),
        partitioning: PartitioningConfig::default(),
        format_options: HashMap::new(),
        validation_enabled: true,
        overwrite_existing: false,
        create_directories: true,
        file_permissions: None,
        buffer_size: 8192,
        max_file_size: None,
    };
    
    // Create generator
    let mut generator = factory.create_generator("csv", config.clone()).await?;
    
    // Generate output (assuming you have processed data)
    let processed_data = ProcessedData::new(); // Your processed data here
    let result = generator.generate(processed_data, config).await?;
    
    println!("Generated {} records in {} ms", 
             result.records_written, 
             result.generation_time_ms);
    
    Ok(())
}
```

### Advanced Configuration

```rust
use rusty::output::*;
use std::collections::HashMap;

// Configure CSV output with custom options
let mut csv_options = HashMap::new();
csv_options.insert("delimiter".to_string(), "|".to_string());
csv_options.insert("quote_char".to_string(), "'".to_string());
csv_options.insert("include_headers".to_string(), "true".to_string());

let config = OutputConfig {
    output_path: PathBuf::from("output/survey_data.csv"),
    format: "csv".to_string(),
    compression: CompressionConfig {
        enabled: true,
        algorithm: CompressionAlgorithm::Gzip,
        level: 6,
        dictionary: None,
    },
    partitioning: PartitioningConfig {
        enabled: true,
        strategy: PartitionStrategy::BySize,
        partition_size: 100_000_000, // 100MB
        naming_strategy: PartitionNamingStrategy::Sequential,
    },
    format_options: csv_options,
    validation_enabled: true,
    overwrite_existing: false,
    create_directories: true,
    file_permissions: Some(0o644),
    buffer_size: 65536,
    max_file_size: Some(1_000_000_000), // 1GB
};
```

### Using with Registry

```rust
use rusty::output::{DefaultOutputRegistry, CsvOutputGenerator};

// Create registry
let mut registry = DefaultOutputRegistry::new();

// Register custom generator
let csv_generator = Box::new(CsvOutputGenerator::new());
registry.register_generator("custom_csv".to_string(), csv_generator).await?;

// Use registered generator
let generator = registry.get_generator("custom_csv")?;
let stats = generator.stats();
println!("Generator has processed {} records", stats.records_processed);
```

## Performance Considerations

### Memory Management
- **Streaming Processing**: Large datasets are processed in chunks to minimize memory usage
- **Buffer Management**: Configurable buffer sizes for optimal I/O performance
- **Memory Pools**: Reuse of buffers and objects to reduce garbage collection pressure

### I/O Optimization
- **Async I/O**: All I/O operations are asynchronous for better concurrency
- **Batch Writing**: Records are written in batches for better throughput
- **Compression**: Optional compression reduces I/O overhead for large files

### Partitioning Strategies
- **Size-based**: Split files when they reach a certain size
- **Count-based**: Split files after a certain number of records
- **Time-based**: Split files based on timestamp fields
- **Field-based**: Split files based on specific field values

## Error Handling

The output layer provides comprehensive error handling:

### Error Types
- **Configuration Errors**: Invalid configuration parameters
- **I/O Errors**: File system and network I/O failures
- **Format Errors**: Data format validation failures
- **Resource Errors**: Memory and disk space limitations

### Error Recovery
- **Retry Logic**: Automatic retry for transient errors
- **Fallback Strategies**: Alternative output paths when primary fails
- **Partial Recovery**: Continue processing after non-critical errors
- **Error Reporting**: Detailed error context and suggestions

## Monitoring and Statistics

### OutputStats
Comprehensive statistics for monitoring:

```rust
pub struct OutputStats {
    pub records_processed: u64,
    pub bytes_written: u64,
    pub files_created: u64,
    pub generation_time_ms: u64,
    pub compression_ratio: f64,
    pub error_count: u64,
    pub last_generation_time: Option<SystemTime>,
}
```

### Metrics Collection
- **Throughput Metrics**: Records per second, bytes per second
- **Performance Metrics**: Generation time, compression ratios
- **Error Metrics**: Error rates, error types
- **Resource Metrics**: Memory usage, disk usage

## Security Considerations

### File System Security
- **Path Validation**: Prevent directory traversal attacks
- **Permission Management**: Proper file permissions and ownership
- **Secure Temporary Files**: Safe handling of temporary files

### Data Security
- **Sensitive Data Handling**: Automatic detection and masking of sensitive data
- **Encryption**: Optional encryption for sensitive outputs
- **Audit Logging**: Comprehensive logging of all output operations

## Testing

The output layer includes comprehensive testing:

### Unit Tests
- Individual component testing
- Configuration validation testing
- Error handling testing

### Integration Tests
- End-to-end output generation testing
- Performance benchmarking
- Format compatibility testing

### Property-Based Tests
- Configuration property testing
- Data integrity testing
- Performance characteristic testing

## Extension Points

### Custom Formats
Add new output formats by implementing the `FormatWriter` trait:

```rust
pub struct CustomFormatWriter {
    // Implementation details
}

#[async_trait]
impl FormatWriter for CustomFormatWriter {
    fn format_name(&self) -> &str {
        "custom"
    }
    
    // Implement other required methods...
}
```

### Custom Generators
Create specialized generators by implementing the `OutputGenerator` trait:

```rust
pub struct SpecializedGenerator {
    // Implementation details
}

#[async_trait]
impl OutputGenerator for SpecializedGenerator {
    // Implement required methods...
}
```

## Best Practices

### Configuration
- Use environment-specific configuration files
- Validate configuration before processing
- Provide sensible defaults for all options

### Performance
- Choose appropriate buffer sizes for your data
- Use compression for large files
- Consider partitioning for very large datasets

### Error Handling
- Always handle errors gracefully
- Provide meaningful error messages
- Log errors for debugging and monitoring

### Security
- Validate all file paths
- Use appropriate file permissions
- Consider encryption for sensitive data

## Troubleshooting

### Common Issues
1. **Permission Denied**: Check file system permissions
2. **Disk Space**: Ensure adequate disk space for output
3. **Memory Issues**: Reduce buffer sizes or enable streaming
4. **Format Errors**: Validate data format and configuration

### Debug Mode
Enable debug logging for detailed troubleshooting:

```rust
let config = OutputConfig {
    // ... other config
    debug_mode: true,
    log_level: LogLevel::Debug,
};
```

## Future Enhancements

### Planned Features
- **Cloud Storage Support**: Direct output to cloud storage services
- **Real-time Streaming**: Support for real-time data streaming
- **Advanced Compression**: Additional compression algorithms
- **Schema Evolution**: Support for schema evolution in columnar formats

### Performance Improvements
- **SIMD Optimization**: Use SIMD instructions for data processing
- **GPU Acceleration**: GPU-accelerated compression and encoding
- **Parallel Processing**: Enhanced parallel processing capabilities