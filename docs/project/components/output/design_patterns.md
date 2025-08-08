# Output Module: Design Patterns

This document outlines recommended design patterns for the Output module.

## Core Patterns

### 1. Strategy Pattern for Output Formats
```rust
pub trait OutputFormat {
    fn format(&self, data: Vec<ProcessedData>) -> Result<Vec<u8>>;
    fn file_extension(&self) -> &str;
    fn mime_type(&self) -> &str;
}

pub struct CsvFormat {
    delimiter: char,
    quote_char: char,
    headers: bool,
}

pub struct ParquetFormat {
    compression: Compression,
    row_group_size: usize,
}

pub struct JsonFormat {
    pretty_print: bool,
    schema_validation: bool,
}

impl OutputFormat for CsvFormat {
    fn format(&self, data: Vec<ProcessedData>) -> Result<Vec<u8>> {
        // CSV formatting implementation
    }
}
```

### 2. Factory Pattern for Format Creation
```rust
pub struct OutputFormatFactory;

impl OutputFormatFactory {
    pub fn create_format(format_type: FormatType, config: &FormatConfig) -> Result<Box<dyn OutputFormat>> {
        match format_type {
            FormatType::CSV => Ok(Box::new(CsvFormat::from_config(config)?)),
            FormatType::Parquet => Ok(Box::new(ParquetFormat::from_config(config)?)),
            FormatType::JSON => Ok(Box::new(JsonFormat::from_config(config)?)),
        }
    }
    
    pub fn create_optimal_format(data_characteristics: &DataCharacteristics) -> Box<dyn OutputFormat> {
        match data_characteristics {
            DataCharacteristics { size: Large, analytics_use: true, .. } => {
                Box::new(ParquetFormat::with_compression(Compression::SNAPPY))
            }
            DataCharacteristics { web_api_use: true, .. } => {
                Box::new(JsonFormat::with_pretty_print(true))
            }
            _ => Box::new(CsvFormat::default()),
        }
    }
}
```

### 3. Builder Pattern for Output Configuration
```rust
pub struct OutputGeneratorBuilder {
    formats: Vec<Box<dyn OutputFormat>>,
    destination: Option<String>,
    validation: Option<Box<dyn OutputValidator>>,
    compression: Option<Compression>,
}

impl OutputGeneratorBuilder {
    pub fn new() -> Self {
        Self {
            formats: Vec::new(),
            destination: None,
            validation: None,
            compression: None,
        }
    }
    
    pub fn add_format(mut self, format: Box<dyn OutputFormat>) -> Self {
        self.formats.push(format);
        self
    }
    
    pub fn with_destination(mut self, destination: String) -> Self {
        self.destination = Some(destination);
        self
    }
    
    pub fn with_validation(mut self, validator: Box<dyn OutputValidator>) -> Self {
        self.validation = Some(validator);
        self
    }
    
    pub fn build(self) -> Result<OutputGenerator> {
        if self.formats.is_empty() {
            return Err(OutputError::NoFormatsSpecified);
        }
        
        Ok(OutputGenerator {
            formats: self.formats,
            destination: self.destination.unwrap_or_default(),
            validation: self.validation,
            compression: self.compression,
        })
    }
}
```

### 4. Template Method Pattern for Output Generation
```rust
pub trait OutputTemplate {
    fn prepare_data(&self, data: Vec<ProcessedData>) -> Result<Vec<ProcessedData>>;
    fn validate_data(&self, data: &[ProcessedData]) -> Result<()>;
    fn format_data(&self, data: Vec<ProcessedData>) -> Result<Vec<u8>>;
    fn write_output(&self, formatted_data: Vec<u8>, destination: &str) -> Result<()>;
    
    fn generate_output(&self, data: Vec<ProcessedData>, destination: &str) -> Result<()> {
        let prepared = self.prepare_data(data)?;
        self.validate_data(&prepared)?;
        let formatted = self.format_data(prepared)?;
        self.write_output(formatted, destination)?;
        
        Ok(())
    }
}
```

### 5. Observer Pattern for Output Events
```rust
pub trait OutputObserver {
    fn on_output_started(&self, format: &str, record_count: usize);
    fn on_output_progress(&self, processed: usize, total: usize);
    fn on_output_completed(&self, result: &OutputResult);
    fn on_output_error(&self, error: &OutputError);
}

pub struct OutputNotifier {
    observers: Vec<Box<dyn OutputObserver>>,
}

impl OutputNotifier {
    pub fn notify_started(&self, format: &str, record_count: usize) {
        for observer in &self.observers {
            observer.on_output_started(format, record_count);
        }
    }
    
    pub fn notify_progress(&self, processed: usize, total: usize) {
        for observer in &self.observers {
            observer.on_output_progress(processed, total);
        }
    }
}
```

## Integration Patterns

### Adapter Pattern for Legacy Formats
```rust
pub struct LegacyFormatAdapter {
    legacy_formatter: LegacyFormatter,
}

impl OutputFormat for LegacyFormatAdapter {
    fn format(&self, data: Vec<ProcessedData>) -> Result<Vec<u8>> {
        // Convert modern data to legacy format
        let legacy_data = self.convert_to_legacy_format(data)?;
        
        // Use legacy formatter
        let result = self.legacy_formatter.format(legacy_data)?;
        
        // Convert result back to modern format
        Ok(result)
    }
}
```

### Decorator Pattern for Output Enhancement
```rust
pub struct CompressedOutputDecorator {
    inner: Box<dyn OutputFormat>,
    compression: Compression,
}

impl OutputFormat for CompressedOutputDecorator {
    fn format(&self, data: Vec<ProcessedData>) -> Result<Vec<u8>> {
        let formatted = self.inner.format(data)?;
        self.compress(formatted)
    }
}

pub struct ValidatedOutputDecorator {
    inner: Box<dyn OutputFormat>,
    validator: Box<dyn OutputValidator>,
}

impl OutputFormat for ValidatedOutputDecorator {
    fn format(&self, data: Vec<ProcessedData>) -> Result<Vec<u8>> {
        self.validator.validate(&data)?;
        self.inner.format(data)
    }
}
```

### Chain of Responsibility for Output Processing
```rust
pub trait OutputProcessor {
    fn process(&self, data: Vec<ProcessedData>) -> Result<Vec<ProcessedData>>;
    fn set_next(&mut self, processor: Box<dyn OutputProcessor>);
}

pub struct ValidationProcessor {
    next: Option<Box<dyn OutputProcessor>>,
    validator: Box<dyn OutputValidator>,
}

impl OutputProcessor for ValidationProcessor {
    fn process(&self, data: Vec<ProcessedData>) -> Result<Vec<ProcessedData>> {
        self.validator.validate(&data)?;
        
        if let Some(ref next) = self.next {
            next.process(data)
        } else {
            Ok(data)
        }
    }
}
```