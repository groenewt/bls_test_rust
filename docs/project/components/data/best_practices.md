# Data Module: Best Practices

This document outlines best practices for using the Data module.

## Data Model Design

### 1. Use Strong Typing
```rust
// Good: Strong typing with validation
#[derive(Debug, Clone, Validate)]
pub struct Series {
    #[validate(length(min = 1))]
    pub id: String,
    
    #[validate(length(equal = 2))]
    pub survey_code: String,
    
    pub observations: Vec<Observation>,
}

// Avoid: Weak typing with strings everywhere
```

### 2. Implement Validation Early
```rust
impl Series {
    pub fn new(id: String, survey_code: String) -> Result<Self> {
        let series = Self {
            id,
            survey_code,
            observations: Vec::new(),
        };
        
        // Validate immediately upon creation
        series.validate()?;
        Ok(series)
    }
}
```

## Data I/O Best Practices

### 1. Choose Appropriate Reader Strategy
```rust
pub fn choose_reader(file_size: u64) -> Box<dyn DataReader> {
    match file_size {
        0..=100_000_000 => Box::new(FileReader::new()),      // < 100MB
        100_000_001..=1_000_000_000 => Box::new(MmapReader::new()), // 100MB-1GB
        _ => Box::new(StreamingReader::new()),               // > 1GB
    }
}
```

### 2. Use Streaming for Large Datasets
```rust
pub fn process_large_file(path: &Path) -> Result<()> {
    let reader = StreamingReader::new();
    let mut stream = reader.stream_from_file(path)?;
    
    while let Some(chunk) = stream.next_chunk(10000)? {
        process_chunk(chunk)?;
        // Process in chunks to avoid memory issues
    }
    
    Ok(())
}
```

### 3. Validate Data During Reading
```rust
pub fn read_and_validate(path: &Path) -> Result<Vec<Observation>> {
    let reader = FileReader::new();
    let raw_data = reader.read_from_file(path)?;
    
    let mut validated_data = Vec::new();
    for item in raw_data {
        match item.validate() {
            Ok(obs) => validated_data.push(obs),
            Err(e) => {
                log::warn!("Invalid observation skipped: {}", e);
                continue;
            }
        }
    }
    
    Ok(validated_data)
}
```

## Performance Optimization

### 1. Use Memory Mapping for Large Files
```rust
pub fn read_large_file_efficiently(path: &Path) -> Result<Vec<Series>> {
    let reader = MmapReader::new();
    reader.read_from_file(path) // Uses memory mapping internally
}
```

### 2. Batch Operations
```rust
pub fn write_data_efficiently(data: Vec<Observation>) -> Result<()> {
    const BATCH_SIZE: usize = 1000;
    let writer = CsvWriter::new();
    
    for batch in data.chunks(BATCH_SIZE) {
        writer.write_batch(batch)?;
    }
    
    Ok(())
}
```

## Error Handling

### 1. Provide Context in Errors
```rust
pub fn read_survey_data(survey_code: &str) -> Result<Vec<Series>> {
    let path = format!("data/{}.series", survey_code);
    
    FileReader::new()
        .read_from_file(&path)
        .with_context(|| format!("Failed to read survey data for '{}'", survey_code))
}
```

### 2. Handle Partial Failures Gracefully
```rust
pub fn process_observations(observations: Vec<Observation>) -> (Vec<ProcessedData>, Vec<DataError>) {
    let mut results = Vec::new();
    let mut errors = Vec::new();
    
    for obs in observations {
        match process_observation(obs) {
            Ok(processed) => results.push(processed),
            Err(e) => errors.push(e),
        }
    }
    
    (results, errors)
}
```

## Testing

### 1. Use Test Data Builders
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_series() -> Series {
        Series::builder()
            .id("TEST_SERIES_001")
            .survey_code("TS")
            .title("Test Series")
            .build()
            .unwrap()
    }
    
    #[test]
    fn test_series_validation() {
        let series = test_series();
        assert!(series.validate().is_ok());
    }
}
```

### 2. Test Edge Cases
```rust
#[test]
fn test_empty_data_handling() {
    let empty_data: Vec<Observation> = Vec::new();
    let result = process_observations(empty_data);
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_invalid_data_handling() {
    let invalid_obs = Observation::new("", 0.0); // Invalid
    assert!(invalid_obs.validate().is_err());
}
```

## Security

### 1. Validate File Paths
```rust
pub fn read_file_safely(path: &Path) -> Result<Vec<u8>> {
    // Ensure path is within allowed directories
    let canonical = path.canonicalize()?;
    if !canonical.starts_with("/allowed/data/directory") {
        return Err(DataError::InvalidPath);
    }
    
    std::fs::read(canonical).map_err(DataError::IoError)
}
```

### 2. Sanitize Data for Output
```rust
impl Observation {
    pub fn sanitize_for_export(&self) -> Observation {
        let mut sanitized = self.clone();
        
        // Remove or mask sensitive information
        if sanitized.is_sensitive() {
            sanitized.value = 0.0;
            sanitized.add_flag("SANITIZED");
        }
        
        sanitized
    }
}
```