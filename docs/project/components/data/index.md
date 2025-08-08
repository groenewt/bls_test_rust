# Data Layer Documentation

Welcome to the Data Layer documentation for the Rusty BLS Data Processing system. This component handles all data models, reading, and writing operations across the system.

## Overview

The Data Layer is responsible for:
- Defining data models for BLS survey data (Series, Observations, Lookups)
- Reading data from various file formats and sources
- Writing processed data to multiple output formats
- Validating data integrity and consistency
- Managing data transformations and conversions

## Architecture

```mermaid
graph TB
    subgraph "Data Sources"
        SeriesFiles[Series Files]
        DataFiles[Data Files]
        LookupFiles[Lookup Files]
        ExternalAPIs[External APIs]
    end
    
    subgraph "Data Models"
        Series[Series Model]
        Observation[Observation Model]
        Lookup[Lookup Model]
        Survey[Survey Model]
    end
    
    subgraph "Data Readers"
        FileReader[File Reader]
        MMapReader[Memory-Mapped Reader]
        StreamReader[Stream Reader]
        ReaderFactory[Reader Factory]
    end
    
    subgraph "Data Writers"
        CSVWriter[CSV Writer]
        ParquetWriter[Parquet Writer]
        JSONWriter[JSON Writer]
        WriterFactory[Writer Factory]
    end
    
    subgraph "Data Operations"
        Validator[Data Validator]
        Transformer[Data Transformer]
        Aggregator[Data Aggregator]
        Merger[Data Merger]
    end
    
    SeriesFiles --> FileReader
    DataFiles --> FileReader
    LookupFiles --> FileReader
    ExternalAPIs --> StreamReader
    
    FileReader --> Series
    FileReader --> Observation
    FileReader --> Lookup
    MMapReader --> Series
    MMapReader --> Observation
    
    Series --> Survey
    Observation --> Survey
    Lookup --> Survey
    
    Survey --> Validator
    Survey --> Transformer
    Survey --> Aggregator
    Survey --> Merger
    
    Survey --> CSVWriter
    Survey --> ParquetWriter
    Survey --> JSONWriter
    
    ReaderFactory --> FileReader
    ReaderFactory --> MMapReader
    ReaderFactory --> StreamReader
    
    WriterFactory --> CSVWriter
    WriterFactory --> ParquetWriter
    WriterFactory --> JSONWriter
```

## Data Models

### Series Model

The Series model represents metadata about data series in BLS surveys:

```mermaid
classDiagram
    class Series {
        +String series_id
        +String survey_code
        +String area_code
        +String item_code
        +String data_type_code
        +String seasonal
        +String periodicity_code
        +String base_code
        +DateTime begin_year
        +DateTime end_year
        +DateTime begin_period
        +DateTime end_period
        +validate() bool
        +to_json() String
        +from_csv_row(row) Series
    }
    
    class Observation {
        +String series_id
        +i32 year
        +String period
        +f64 value
        +String footnote_codes
        +validate() bool
        +is_annual() bool
        +get_date() DateTime
    }
    
    class Lookup {
        +String code
        +String text
        +String display_level
        +String selectable
        +String sort_sequence
        +validate() bool
    }
    
    class Survey {
        +String code
        +String name
        +Vec~Series~ series
        +Vec~Observation~ observations
        +HashMap~String,Lookup~ lookups
        +validate() bool
        +get_series_count() usize
        +get_observation_count() usize
    }
    
    Survey --> Series
    Survey --> Observation
    Survey --> Lookup
    Series --> Observation
```

## Data Reading Process

```mermaid
sequenceDiagram
    participant App as Application
    participant Factory as Reader Factory
    participant Reader as Data Reader
    participant Parser as Data Parser
    participant Validator as Data Validator
    participant Model as Data Model
    
    App->>Factory: Create reader for file type
    Factory->>Reader: Initialize appropriate reader
    App->>Reader: Read data file
    Reader->>Parser: Parse file content
    Parser->>Validator: Validate parsed data
    
    alt Data Valid
        Validator->>Model: Create data models
        Model->>App: Return structured data
    else Data Invalid
        Validator->>App: Return validation errors
    end
```

## Data Writing Process

```mermaid
sequenceDiagram
    participant App as Application
    participant Factory as Writer Factory
    participant Writer as Data Writer
    participant Formatter as Data Formatter
    participant Compressor as Data Compressor
    participant File as Output File
    
    App->>Factory: Create writer for format
    Factory->>Writer: Initialize appropriate writer
    App->>Writer: Write data models
    Writer->>Formatter: Format data for output
    Formatter->>Compressor: Apply compression if needed
    Compressor->>File: Write to output file
    File->>App: Confirm write completion
```

## Key Features

### 🏗️ Flexible Data Models
- **Strongly Typed**: Type-safe data models with validation
- **Extensible**: Easy to add new fields and data types
- **Serializable**: Support for JSON, CSV, and binary formats
- **Validated**: Built-in data validation and consistency checks

### 📖 Multiple Reader Types
- **File Reader**: Standard file-based reading for small to medium files
- **Memory-Mapped Reader**: Efficient reading for large files
- **Stream Reader**: Real-time data processing from streams
- **Factory Pattern**: Automatic reader selection based on data size and type

### ✍️ Multiple Writer Types
- **CSV Writer**: Human-readable format for analysis
- **Parquet Writer**: Columnar format for efficient storage and querying
- **JSON Writer**: Web-friendly format for APIs
- **Factory Pattern**: Automatic writer selection based on configuration

### 🔍 Data Validation
- **Schema Validation**: Ensure data conforms to expected schemas
- **Range Validation**: Validate numeric values are within expected ranges
- **Consistency Validation**: Check data consistency across related records
- **Custom Validation**: Support for survey-specific validation rules

## Data Flow Architecture

```mermaid
flowchart TD
    Start([Start Data Processing]) --> DetectFormat{Detect File Format}
    DetectFormat -->|Series File| ReadSeries[Read Series Data]
    DetectFormat -->|Data File| ReadData[Read Observation Data]
    DetectFormat -->|Lookup File| ReadLookup[Read Lookup Data]
    
    ReadSeries --> ValidateSeries{Validate Series}
    ReadData --> ValidateData{Validate Observations}
    ReadLookup --> ValidateLookup{Validate Lookups}
    
    ValidateSeries -->|Valid| StoreSeries[Store Series Models]
    ValidateData -->|Valid| StoreData[Store Observation Models]
    ValidateLookup -->|Valid| StoreLookup[Store Lookup Models]
    
    ValidateSeries -->|Invalid| SeriesError[Series Validation Error]
    ValidateData -->|Invalid| DataError[Data Validation Error]
    ValidateLookup -->|Invalid| LookupError[Lookup Validation Error]
    
    StoreSeries --> MergeModels[Merge All Models]
    StoreData --> MergeModels
    StoreLookup --> MergeModels
    
    MergeModels --> CreateSurvey[Create Survey Model]
    CreateSurvey --> FinalValidation{Final Validation}
    
    FinalValidation -->|Valid| ReadyForProcessing([Ready for Processing])
    FinalValidation -->|Invalid| FinalError[Final Validation Error]
    
    SeriesError --> End([End with Error])
    DataError --> End
    LookupError --> End
    FinalError --> End
```

## Reader Implementations

### File Reader Strategy

```mermaid
graph LR
    subgraph "File Reader Strategy Selection"
        FileSize{File Size}
        FileSize -->|< 100MB| InMemory[In-Memory Reader]
        FileSize -->|100MB - 1GB| Buffered[Buffered Reader]
        FileSize -->|> 1GB| MMap[Memory-Mapped Reader]
    end
    
    subgraph "Reading Process"
        InMemory --> Parse1[Parse All at Once]
        Buffered --> Parse2[Parse in Chunks]
        MMap --> Parse3[Parse on Demand]
    end
    
    Parse1 --> Validate[Validate Data]
    Parse2 --> Validate
    Parse3 --> Validate
```

### Writer Strategy Selection

```mermaid
graph LR
    subgraph "Writer Strategy Selection"
        OutputFormat{Output Format}
        OutputFormat -->|CSV| CSVConfig[CSV Configuration]
        OutputFormat -->|Parquet| ParquetConfig[Parquet Configuration]
        OutputFormat -->|JSON| JSONConfig[JSON Configuration]
    end
    
    subgraph "Writing Process"
        CSVConfig --> CSVWrite[Write CSV with Compression]
        ParquetConfig --> ParquetWrite[Write Parquet with Optimization]
        JSONConfig --> JSONWrite[Write JSON with Formatting]
    end
    
    CSVWrite --> Complete[Writing Complete]
    ParquetWrite --> Complete
    JSONWrite --> Complete
```

## Data Transformation Pipeline

```mermaid
flowchart TD
    RawData[Raw Data] --> TypeConversion[Type Conversion]
    TypeConversion --> DateNormalization[Date Normalization]
    DateNormalization --> ValueValidation[Value Validation]
    ValueValidation --> SeasonalAdjustment[Seasonal Adjustment]
    SeasonalAdjustment --> UnitConversion[Unit Conversion]
    UnitConversion --> QualityChecks[Quality Checks]
    QualityChecks --> EnrichmentLookup[Enrichment with Lookups]
    EnrichmentLookup --> FinalValidation[Final Validation]
    FinalValidation --> ProcessedData[Processed Data]
    
    TypeConversion -->|Error| TransformError[Transformation Error]
    DateNormalization -->|Error| TransformError
    ValueValidation -->|Error| TransformError
    SeasonalAdjustment -->|Error| TransformError
    UnitConversion -->|Error| TransformError
    QualityChecks -->|Error| TransformError
    EnrichmentLookup -->|Error| TransformError
    FinalValidation -->|Error| TransformError
```

## Performance Optimization

### Memory Management

The Data Layer implements several memory optimization strategies:

- **Lazy Loading**: Load data only when needed
- **Memory Pooling**: Reuse memory allocations for similar data structures
- **Streaming Processing**: Process large datasets without loading everything into memory
- **Compression**: Use compressed data structures where appropriate

### I/O Optimization

- **Parallel Reading**: Read multiple files concurrently
- **Buffered I/O**: Use appropriate buffer sizes for different file types
- **Memory Mapping**: Use memory-mapped files for large datasets
- **Async Operations**: Non-blocking I/O operations where possible

## Integration with Other Components

### Configuration Integration
The Data Layer receives configuration from the Configuration System:
- File paths and naming patterns
- Data validation rules and thresholds
- Reader and writer preferences
- Performance tuning parameters

### Processing Integration
The Data Layer provides data to the Processing System:
- Structured data models ready for processing
- Data validation results and quality metrics
- Metadata about data characteristics
- Processing hints and recommendations

### Error Handling Integration
The Data Layer integrates with the Error Handling System:
- Detailed error reporting for data issues
- Context information for debugging
- Error recovery strategies
- Data quality metrics and alerts

## Testing Strategy

### Unit Tests
- Test individual data model validation
- Test reader and writer functionality
- Test data transformation operations
- Test error handling scenarios

### Integration Tests
- Test end-to-end data processing workflows
- Test with real BLS data files
- Test performance with large datasets
- Test error recovery mechanisms

### Performance Tests
- Benchmark reading and writing operations
- Test memory usage with large datasets
- Test concurrent processing capabilities
- Validate optimization strategies

## Best Practices

### Data Modeling
- Use strong typing for all data fields
- Implement comprehensive validation
- Design for extensibility and evolution
- Document data relationships clearly

### Performance
- Choose appropriate reader/writer strategies
- Monitor memory usage and optimize accordingly
- Use parallel processing where beneficial
- Profile and optimize critical paths

### Error Handling
- Validate data at multiple stages
- Provide detailed error messages
- Implement graceful error recovery
- Log data quality issues for analysis

### Security
- Validate all input data
- Sanitize data before processing
- Protect against malformed data attacks
- Audit data access and modifications

## Troubleshooting

### Common Issues

1. **Memory Issues with Large Files**
   - Use memory-mapped readers for files > 1GB
   - Enable streaming processing
   - Monitor memory usage during processing

2. **Data Validation Failures**
   - Check data format specifications
   - Verify field types and ranges
   - Review validation rules configuration

3. **Performance Issues**
   - Profile I/O operations
   - Check for memory leaks
   - Optimize data structures
   - Use appropriate concurrency levels

## Contributing

When contributing to the Data Layer:

1. Follow the existing data model patterns
2. Add comprehensive validation for new data types
3. Write tests for all data operations
4. Update documentation for new features
5. Consider performance implications of changes

## Further Reading

- [Data Model Reference](models.md)
- [Reader Implementation Guide](readers.md)
- [Writer Implementation Guide](writers.md)
- [Performance Tuning Guide](performance.md)

---

For questions about the Data Layer, please refer to the troubleshooting section or create an issue in the project repository.