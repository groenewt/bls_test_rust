# Output Module: Best Practices

This document outlines best practices for using the Output module in the Rusty BLS Data Processing system.

## Output Format Design

### 1. Use Appropriate Output Formats

Choose the right output format for your use case:

✅ **Good**:
```rust
// For analytics and big data processing
let parquet_config = ParquetConfig {
    compression: CompressionType::Snappy,
    row_group_size: 100_000,
    enable_statistics: true,
};

// For spreadsheet compatibility
let csv_config = CsvConfig {
    delimiter: b',',
    quote_char: b'"',
    escape_char: Some(b'\\'),
    headers: true,
};

// For web APIs and data exchange
let json_config = JsonConfig {
    pretty_print: false,
    include_metadata: true,
    date_format: DateFormat::Iso8601,
};
```

❌ **Bad**:
```rust
// Using CSV for everything
let output = CsvOutput::new(); // Not optimal for large datasets
```

### 2. Configure Output Validation

Always validate output data before writing:

✅ **Good**:
```rust
let validator = OutputValidator::new()
    .with_schema_validation(true)
    .with_data_type_checks(true)
    .with_range_validation(true);

let output = CsvOutput::new()
    .with_validator(validator)
    .with_error_handling(ErrorHandling::StopOnError);
```

❌ **Bad**:
```rust
let output = CsvOutput::new(); // No validation
```

### 3. Use Structured Output Configuration

Use structured configuration rather than hardcoded values:

✅ **Good**:
```rust
#[derive(Deserialize)]
struct OutputConfig {
    format: OutputFormat,
    destination: OutputDestination,
    validation: ValidationConfig,
    compression: Option<CompressionConfig>,
}

fn create_output(config: &OutputConfig) -> Result<Box<dyn OutputGenerator>> {
    OutputFactory::create(config)
}
```

❌ **Bad**:
```rust
fn create_output() -> CsvOutput {
    CsvOutput::new()
        .with_delimiter(',')
        .with_headers(true) // Hardcoded configuration
}
```

## Output Generation Patterns

### 1. Use Streaming for Large Datasets

Use streaming output for large datasets to manage memory usage:

✅ **Good**:
```rust
fn write_large_dataset<W: Write>(data: impl Iterator<Item = Record>, writer: W) -> Result<()> {
    let mut output = CsvOutput::new(writer);
    
    for record in data {
        output.write_record(&record)?;
    }
    
    output.flush()?;
    Ok(())
}
```

❌ **Bad**:
```rust
fn write_large_dataset<W: Write>(data: Vec<Record>, writer: W) -> Result<()> {
    let output = CsvOutput::new(writer);
    output.write_all(&data)?; // Loads everything into memory
    Ok(())
}
```

### 2. Handle Output Errors Gracefully

Implement proper error handling for output operations:

✅ **Good**:
```rust
fn write_survey_data(survey: &Survey, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)
        .with_context(format!("Failed to create output file: {}", output_path.display()))?;
    
    let mut writer = CsvOutput::new(file)
        .with_error_handling(ErrorHandling::ContinueOnError)
        .with_max_errors(100);
    
    for series in &survey.series {
        if let Err(e) = writer.write_series(series) {
            log::warn!("Failed to write series {}: {}", series.id, e);
        }
    }
    
    writer.finalize()
        .with_context("Failed to finalize output file")?;
    
    Ok(())
}
```

❌ **Bad**:
```rust
fn write_survey_data(survey: &Survey, output_path: &Path) -> Result<()> {
    let file = File::create(output_path)?;
    let mut writer = CsvOutput::new(file);
    
    for series in &survey.series {
        writer.write_series(series)?; // Stops on first error
    }
    
    Ok(())
}
```

### 3. Use Appropriate Buffering

Configure appropriate buffering for your output operations:

✅ **Good**:
```rust
fn create_buffered_output<W: Write>(writer: W, buffer_size: usize) -> Result<BufferedOutput<W>> {
    let buffered_writer = BufWriter::with_capacity(buffer_size, writer);
    Ok(BufferedOutput::new(buffered_writer))
}

// For large files
let output = create_buffered_output(file, 64 * 1024)?; // 64KB buffer

// For small files
let output = create_buffered_output(file, 8 * 1024)?; // 8KB buffer
```

❌ **Bad**:
```rust
let output = CsvOutput::new(file); // No buffering consideration
```

## Performance Optimization

### 1. Use Parallel Processing for Multiple Files

Process multiple output files in parallel when possible:

✅ **Good**:
```rust
use rayon::prelude::*;

fn write_multiple_surveys(surveys: &[Survey], output_dir: &Path) -> Result<()> {
    surveys.par_iter()
        .map(|survey| {
            let output_path = output_dir.join(format!("{}.csv", survey.code));
            write_survey_data(survey, &output_path)
        })
        .collect::<Result<Vec<_>>>()?;
    
    Ok(())
}
```

❌ **Bad**:
```rust
fn write_multiple_surveys(surveys: &[Survey], output_dir: &Path) -> Result<()> {
    for survey in surveys {
        let output_path = output_dir.join(format!("{}.csv", survey.code));
        write_survey_data(survey, &output_path)?; // Sequential processing
    }
    Ok(())
}
```

### 2. Optimize Memory Usage

Minimize memory allocations in output operations:

✅ **Good**:
```rust
struct OutputBuffer {
    buffer: Vec<u8>,
}

impl OutputBuffer {
    fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(8192), // Pre-allocate
        }
    }
    
    fn write_record(&mut self, record: &Record) -> Result<()> {
        self.buffer.clear(); // Reuse buffer
        record.serialize_into(&mut self.buffer)?;
        // Write buffer to output
        Ok(())
    }
}
```

❌ **Bad**:
```rust
fn write_record(record: &Record) -> Result<String> {
    let serialized = record.serialize()?; // New allocation each time
    Ok(serialized)
}
```

### 3. Use Compression Appropriately

Apply compression based on output format and use case:

✅ **Good**:
```rust
// For Parquet files
let parquet_config = ParquetConfig {
    compression: CompressionType::Snappy, // Good balance of speed/size
    enable_dictionary: true,
    enable_statistics: true,
};

// For CSV files going over network
let csv_config = CsvConfig {
    compression: Some(CompressionType::Gzip),
    compression_level: 6, // Balanced compression
};

// For JSON APIs
let json_config = JsonConfig {
    compression: None, // Usually handled by HTTP layer
    minify: true,
};
```

❌ **Bad**:
```rust
// Using maximum compression for everything
let config = OutputConfig {
    compression: Some(CompressionType::Bzip2), // Slow compression
    compression_level: 9, // Maximum compression, slow
};
```

## Error Handling

### 1. Implement Retry Logic for Transient Errors

Implement retry logic for network and I/O errors:

✅ **Good**:
```rust
async fn write_to_remote(data: &[u8], url: &str) -> Result<()> {
    let mut attempts = 0;
    let max_attempts = 3;
    
    loop {
        match upload_data(data, url).await {
            Ok(()) => return Ok(()),
            Err(e) if e.is_transient() && attempts < max_attempts => {
                attempts += 1;
                let delay = Duration::from_millis(100 * 2_u64.pow(attempts));
                tokio::time::sleep(delay).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

❌ **Bad**:
```rust
async fn write_to_remote(data: &[u8], url: &str) -> Result<()> {
    upload_data(data, url).await // No retry logic
}
```

### 2. Validate Output Before Writing

Always validate data before writing to output:

✅ **Good**:
```rust
fn write_validated_data<W: Write>(data: &[Record], writer: W) -> Result<()> {
    // Validate data first
    for (i, record) in data.iter().enumerate() {
        record.validate()
            .with_context(format!("Invalid record at index {}", i))?;
    }
    
    // Write validated data
    let mut output = CsvOutput::new(writer);
    for record in data {
        output.write_record(record)?;
    }
    
    Ok(())
}
```

❌ **Bad**:
```rust
fn write_data<W: Write>(data: &[Record], writer: W) -> Result<()> {
    let mut output = CsvOutput::new(writer);
    for record in data {
        output.write_record(record)?; // No validation
    }
    Ok(())
}
```

### 3. Provide Detailed Error Context

Include detailed context in error messages:

✅ **Good**:
```rust
fn write_survey_output(survey: &Survey, config: &OutputConfig) -> Result<()> {
    let output_path = &config.output_path;
    
    let file = File::create(output_path)
        .with_context(format!(
            "Failed to create output file '{}' for survey '{}'",
            output_path.display(),
            survey.code
        ))?;
    
    let writer = OutputFactory::create(&config.format, file)
        .with_context(format!(
            "Failed to create {} writer for survey '{}'",
            config.format,
            survey.code
        ))?;
    
    writer.write_survey(survey)
        .with_context(format!(
            "Failed to write survey '{}' to '{}'",
            survey.code,
            output_path.display()
        ))?;
    
    Ok(())
}
```

❌ **Bad**:
```rust
fn write_survey_output(survey: &Survey, config: &OutputConfig) -> Result<()> {
    let file = File::create(&config.output_path)?;
    let writer = OutputFactory::create(&config.format, file)?;
    writer.write_survey(survey)?; // No context
    Ok(())
}
```

## Testing

### 1. Test Different Output Formats

Test your code with different output formats:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_csv_output() {
        let data = create_test_data();
        let mut buffer = Vec::new();
        
        let result = write_csv_data(&data, &mut buffer);
        assert!(result.is_ok());
        
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("header1,header2"));
    }
    
    #[test]
    fn test_json_output() {
        let data = create_test_data();
        let mut buffer = Vec::new();
        
        let result = write_json_data(&data, &mut buffer);
        assert!(result.is_ok());
        
        let output: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert!(output.is_array());
    }
    
    #[test]
    fn test_parquet_output() {
        let data = create_test_data();
        let mut buffer = Vec::new();
        
        let result = write_parquet_data(&data, &mut buffer);
        assert!(result.is_ok());
        assert!(!buffer.is_empty());
    }
}
```

### 2. Test Error Conditions

Test error handling in output operations:

```rust
#[test]
fn test_invalid_output_path() {
    let data = create_test_data();
    let invalid_path = Path::new("/invalid/path/output.csv");
    
    let result = write_data_to_file(&data, invalid_path);
    assert!(result.is_err());
    
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Failed to create output file"));
}

#[test]
fn test_write_permission_denied() {
    let data = create_test_data();
    let readonly_path = create_readonly_file();
    
    let result = write_data_to_file(&data, &readonly_path);
    assert!(result.is_err());
}
```

### 3. Test Performance Characteristics

Test performance with different data sizes:

```rust
#[test]
fn test_large_dataset_performance() {
    let large_data = create_large_test_data(1_000_000);
    let mut buffer = Vec::new();
    
    let start = std::time::Instant::now();
    let result = write_csv_data(&large_data, &mut buffer);
    let duration = start.elapsed();
    
    assert!(result.is_ok());
    assert!(duration < Duration::from_secs(10)); // Should complete within 10 seconds
}
```

## Configuration

### 1. Use Environment-Specific Configurations

Configure output settings based on environment:

```rust
#[derive(Deserialize)]
struct OutputConfig {
    format: OutputFormat,
    compression: Option<CompressionType>,
    buffer_size: usize,
    max_file_size: Option<u64>,
    validation: ValidationConfig,
}

impl OutputConfig {
    fn for_environment(env: &str) -> Result<Self> {
        match env {
            "development" => Ok(Self {
                format: OutputFormat::Csv,
                compression: None,
                buffer_size: 8192,
                max_file_size: None,
                validation: ValidationConfig::strict(),
            }),
            "production" => Ok(Self {
                format: OutputFormat::Parquet,
                compression: Some(CompressionType::Snappy),
                buffer_size: 65536,
                max_file_size: Some(1_000_000_000), // 1GB
                validation: ValidationConfig::standard(),
            }),
            _ => Err(ConfigError::InvalidEnvironment(env.to_string()).into()),
        }
    }
}
```

### 2. Validate Configuration

Always validate output configuration:

```rust
impl OutputConfig {
    fn validate(&self) -> Result<()> {
        if self.buffer_size == 0 {
            return Err(ConfigError::invalid_value {
                key: "buffer_size".to_string(),
                value: self.buffer_size.to_string(),
                message: "Buffer size must be greater than 0".to_string(),
            }.into());
        }
        
        if let Some(max_size) = self.max_file_size {
            if max_size < 1024 {
                return Err(ConfigError::invalid_value {
                    key: "max_file_size".to_string(),
                    value: max_size.to_string(),
                    message: "Maximum file size must be at least 1KB".to_string(),
                }.into());
            }
        }
        
        Ok(())
    }
}
```

## Security

### 1. Sanitize Output Paths

Always sanitize and validate output paths:

```rust
fn sanitize_output_path(path: &Path, base_dir: &Path) -> Result<PathBuf> {
    let canonical_path = path.canonicalize()
        .map_err(|_| OutputError::InvalidPath(path.to_path_buf()))?;
    
    let canonical_base = base_dir.canonicalize()
        .map_err(|_| OutputError::InvalidBasePath(base_dir.to_path_buf()))?;
    
    if !canonical_path.starts_with(&canonical_base) {
        return Err(OutputError::PathTraversalAttempt {
            path: path.to_path_buf(),
            base: base_dir.to_path_buf(),
        }.into());
    }
    
    Ok(canonical_path)
}
```

### 2. Limit Output File Sizes

Implement file size limits to prevent disk space exhaustion:

```rust
struct SizeLimitedWriter<W: Write> {
    inner: W,
    written: u64,
    limit: u64,
}

impl<W: Write> Write for SizeLimitedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written + buf.len() as u64 > self.limit {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "Output size limit exceeded"
            ));
        }
        
        let written = self.inner.write(buf)?;
        self.written += written as u64;
        Ok(written)
    }
    
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
```

This document provides comprehensive best practices for using the Output module effectively, securely, and efficiently in the Rusty BLS Data Processing system.