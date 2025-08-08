# Data Module: Integration Guidelines

This document provides integration guidelines for the Data module.

## Overview

The Data module provides the foundation for all data operations in the Rusty BLS Data Processing system.

## Integration Patterns

### Basic Data Access
```rust
use crate::data::{DataReader, ReaderFactory};

pub struct DataProcessor {
    reader: Box<dyn DataReader>,
}

impl DataProcessor {
    pub fn new(reader_type: &str) -> Result<Self> {
        let reader = ReaderFactory::create_reader(reader_type)?;
        Ok(Self { reader })
    }
}
```

### Configuration-Driven Data Handling
```rust
use crate::config::DataConfig;
use crate::data::{ReaderFactory, WriterFactory};

impl DataProcessor {
    pub fn from_config(config: &DataConfig) -> Result<Self> {
        let reader = ReaderFactory::create_from_config(&config.reader)?;
        let writer = WriterFactory::create_from_config(&config.writer)?;
        
        Ok(Self { reader, writer })
    }
}
```

## Component Integration Points

### Processing Layer Integration
```rust
use crate::data::model::{Series, Observation};
use crate::processing::ProcessingPipeline;

impl ProcessingPipeline {
    pub fn process_series(&self, series: &Series) -> Result<Vec<Observation>> {
        // Process series data using data models
        let observations = series.get_observations()?;
        self.transform_observations(observations)
    }
}
```

### Output Layer Integration
```rust
use crate::data::writer::{DataWriter, WriterFactory};
use crate::output::OutputGenerator;

impl OutputGenerator {
    pub fn write_data(&self, data: &[Observation]) -> Result<()> {
        for format in &self.config.formats {
            let writer = WriterFactory::create_writer(format)?;
            writer.write_data(data)?;
        }
        Ok(())
    }
}
```

## Error Handling Integration
```rust
use crate::error::{Error, DataError, Result};

pub fn read_survey_data(path: &Path) -> Result<Vec<Series>> {
    let reader = FileReader::new();
    
    reader.read_from_file(path)
        .map_err(|e| Error::Data(DataError::ReadError {
            path: path.to_path_buf(),
            source: Box::new(e),
        }))
}
```

## Best Practices

### 1. Use Factory Pattern for Reader/Writer Creation
```rust
// Create readers and writers using factory pattern
let reader = ReaderFactory::create_reader("mmap")?;
let writer = WriterFactory::create_writer("parquet")?;
```

### 2. Validate Data Early
```rust
pub fn process_data(data: Vec<Observation>) -> Result<Vec<ProcessedData>> {
    // Validate all data before processing
    for obs in &data {
        obs.validate()?;
    }
    
    // Process validated data
    data.into_iter().map(|obs| process_observation(obs)).collect()
}
```

### 3. Use Streaming for Large Datasets
```rust
pub fn process_large_dataset(path: &Path) -> Result<()> {
    let reader = MmapReader::new();
    let mut stream = reader.stream_from_file(path)?;
    
    while let Some(chunk) = stream.next_chunk()? {
        process_chunk(chunk)?;
    }
    
    Ok(())
}
```