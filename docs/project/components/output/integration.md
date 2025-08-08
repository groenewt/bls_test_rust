# Output Module: Integration Guidelines

This document provides integration guidelines for the Output module.

## Overview

The Output module serves as the final stage in the Rusty BLS Data Processing pipeline, generating formatted output from processed data.

## Integration Patterns

### Basic Output Generation
```rust
use crate::output::{OutputGenerator, OutputFormat};

pub struct DataExporter {
    generator: Box<dyn OutputGenerator>,
}

impl DataExporter {
    pub fn new(format: OutputFormat) -> Result<Self> {
        let generator = OutputGeneratorFactory::create(format)?;
        Ok(Self { generator })
    }
    
    pub fn export(&self, data: Vec<ProcessedData>, destination: &str) -> Result<()> {
        self.generator.generate_output(data, destination)
    }
}
```

### Configuration-Driven Output
```rust
use crate::config::OutputConfig;

impl OutputGenerator {
    pub fn from_config(config: &OutputConfig) -> Result<Box<dyn OutputGenerator>> {
        let mut generator = OutputGeneratorBuilder::new();
        
        for format in &config.formats {
            generator = generator.add_format(*format);
        }
        
        generator
            .with_destination(&config.destination)
            .with_compression(config.compression)
            .build()
    }
}
```

## Component Integration Points

### Processing Layer Integration
```rust
use crate::processing::ProcessingResult;

impl OutputPipeline {
    pub fn process_and_export(&self, input_data: Vec<Observation>) -> Result<()> {
        // Process data
        let processed = self.processor.process(input_data)?;
        
        // Generate output
        self.output_generator.generate_output(processed, &self.config.destination)?;
        
        Ok(())
    }
}
```

### Configuration Integration
```rust
use crate::config::{OutputConfig, SurveyConfig};

impl OutputGenerator {
    pub fn for_survey(survey_config: &SurveyConfig) -> Result<Self> {
        let output_config = &survey_config.output;
        
        Self::from_config(output_config)
            .with_survey_metadata(&survey_config.metadata)
            .with_validation_rules(&survey_config.validation)
    }
}
```

### Error Handling Integration
```rust
use crate::error::{Error, OutputError, Result};

pub fn generate_survey_output(data: Vec<ProcessedData>, config: &OutputConfig) -> Result<()> {
    let generator = OutputGenerator::from_config(config)
        .map_err(|e| Error::Output(OutputError::ConfigurationError {
            message: format!("Failed to create output generator: {}", e),
        }))?;
    
    generator.generate_output(data, &config.destination)
        .map_err(|e| Error::Output(OutputError::GenerationError {
            destination: config.destination.clone(),
            source: Box::new(e),
        }))
}
```

## Format-Specific Integration

### CSV Output Integration
```rust
use crate::output::format::CsvFormat;

impl CsvFormat {
    pub fn with_survey_schema(survey_code: &str) -> Result<Self> {
        let schema = SurveySchema::load(survey_code)?;
        
        Ok(Self::new()
            .with_headers(schema.column_names())
            .with_delimiter(schema.delimiter())
            .with_quote_char(schema.quote_char()))
    }
}
```

### Parquet Output Integration
```rust
use crate::output::format::ParquetFormat;

impl ParquetFormat {
    pub fn with_optimization(data_characteristics: &DataCharacteristics) -> Self {
        let mut format = Self::new();
        
        if data_characteristics.is_time_series() {
            format = format.with_row_group_size(100_000);
        }
        
        if data_characteristics.has_high_cardinality() {
            format = format.with_compression(Compression::ZSTD);
        }
        
        format
    }
}
```

## Best Practices

### 1. Use Factory Pattern for Output Creation
```rust
pub fn create_output_generator(config: &OutputConfig) -> Result<Box<dyn OutputGenerator>> {
    OutputGeneratorFactory::create(&config.format)
        .with_destination(&config.destination)
        .with_options(&config.options)
        .build()
}
```

### 2. Validate Output Before Writing
```rust
pub fn generate_validated_output(data: Vec<ProcessedData>, config: &OutputConfig) -> Result<()> {
    // Validate data before formatting
    let validator = OutputValidator::for_format(&config.format);
    validator.validate(&data)?;
    
    // Generate output
    let generator = OutputGenerator::from_config(config)?;
    generator.generate_output(data, &config.destination)
}
```

### 3. Handle Large Datasets with Streaming
```rust
pub fn generate_large_output(data: Vec<ProcessedData>) -> Result<()> {
    let writer = StreamingWriter::new();
    
    for chunk in data.chunks(10_000) {
        let formatted = format_chunk(chunk)?;
        writer.write_chunk(formatted)?;
    }
    
    writer.finalize()
}
```

### 4. Support Multiple Output Destinations
```rust
pub fn generate_multi_destination_output(data: Vec<ProcessedData>, destinations: &[String]) -> Result<()> {
    for destination in destinations {
        let generator = OutputGenerator::for_destination(destination)?;
        generator.generate_output(data.clone(), destination)?;
    }
    
    Ok(())
}
```

## Performance Considerations

### Memory Management
```rust
pub fn generate_memory_efficient_output(data: Vec<ProcessedData>) -> Result<()> {
    let writer = BufferedWriter::new(1024 * 1024); // 1MB buffer
    
    for batch in data.chunks(1000) {
        let formatted = format_batch(batch)?;
        writer.write_batch(formatted)?;
        
        // Flush periodically to manage memory
        if writer.buffer_size() > 10 * 1024 * 1024 { // 10MB
            writer.flush()?;
        }
    }
    
    writer.close()
}
```

### Parallel Output Generation
```rust
use rayon::prelude::*;

pub fn generate_parallel_output(data: Vec<ProcessedData>, formats: &[OutputFormat]) -> Result<()> {
    formats.par_iter()
        .map(|format| {
            let generator = OutputGenerator::for_format(*format)?;
            generator.generate_output(data.clone(), &format.default_destination())
        })
        .collect::<Result<Vec<_>, _>>()?;
    
    Ok(())
}
```