# Data Module

The Data module provides comprehensive data management capabilities for the Rusty BLS Data Processing application. It defines data models, reading mechanisms, writing utilities, and data operations for handling BLS survey data efficiently.

## Overview

The Data module is designed to:

1. Provide structured data models for BLS survey data (series, observations, lookups)
2. Enable flexible data reading from multiple sources and formats
3. Support efficient data writing to various output formats
4. Facilitate data transformations and validations throughout the processing pipeline

## Module Structure

```
src/data/
├── mod.rs           # Module entry point and re-exports
├── model/           # Data models and structures
│   ├── series.rs    # Series data model
│   ├── observation.rs # Observation data model
│   ├── lookup.rs    # Lookup table data model
│   └── survey.rs    # Survey data model
├── reader/          # Data readers
│   ├── traits.rs    # Reader interfaces
│   ├── file_reader.rs # File-based reader implementation
│   ├── mmap_reader.rs # Memory-mapped reader implementation
│   └── factory.rs   # Reader factory
└── writer/          # Data writers
    ├── traits.rs    # Writer interfaces
    ├── csv_writer.rs # CSV writer implementation
    ├── parquet_writer.rs # Parquet writer implementation
    ├── json_writer.rs # JSON writer implementation
    └── factory.rs   # Writer factory
```

## Key Components

### Data Models (`model/`)

The `model/` directory defines structured data representations:

- `Series`: Represents BLS data series with metadata and identifiers
- `Observation`: Individual data points with values and time periods
- `Lookup`: Mapping tables for codes to human-readable descriptions
- `Survey`: Complete survey data structure containing series and observations

### Data Readers (`reader/`)

The `reader/` directory provides data input capabilities:

- `DataReader`: Main data reading interface
- `FileReader`: Standard file-based data reading
- `MmapReader`: Memory-mapped file reading for large datasets
- `ReaderFactory`: Creates appropriate readers based on data size and format

### Data Writers (`writer/`)

The `writer/` directory provides data output capabilities:

- `DataWriter`: Main data writing interface
- `CsvWriter`: CSV format output writer
- `ParquetWriter`: Parquet format output writer
- `JsonWriter`: JSON format output writer
- `WriterFactory`: Creates appropriate writers based on output requirements

## Integration with Other Components

The Data module integrates with other components by:

1. Providing data models used throughout the application
2. Offering flexible data I/O interfaces for different processing strategies
3. Supporting configuration-driven data handling and validation

For more detailed information, see:

- [Tasks and Roadmap](tasks.md)
- [Integration Guidelines](integration.md)
- [Best Practices](best_practices.md)