# Utilities Documentation

Welcome to the Utilities documentation for the Rusty BLS Data Processing system. This component provides essential utility functions and helpers used throughout the system.

## Overview

The Utilities component provides:
- Path manipulation and file system operations
- File handling utilities with safety checks
- Time and date processing functions
- Data formatting and conversion utilities
- Validation helpers and common patterns
- System resource monitoring utilities

## Architecture

```mermaid
graph TB
    subgraph "Utility Categories"
        PathUtils[Path Utilities]
        FileUtils[File Utilities]
        TimeUtils[Time Utilities]
        FormatUtils[Format Utilities]
        ValidationUtils[Validation Utilities]
        SystemUtils[System Utilities]
    end
    
    subgraph "Core Functions"
        PathOps[Path Operations]
        FileOps[File Operations]
        TimeOps[Time Operations]
        FormatOps[Format Operations]
        ValidateOps[Validation Operations]
        SystemOps[System Operations]
    end
    
    subgraph "System Integration"
        Config[Configuration System]
        Data[Data Layer]
        Processing[Processing System]
        Output[Output System]
        Error[Error Handling]
    end
    
    PathUtils --> PathOps
    FileUtils --> FileOps
    TimeUtils --> TimeOps
    FormatUtils --> FormatOps
    ValidationUtils --> ValidateOps
    SystemUtils --> SystemOps
    
    PathOps --> Config
    PathOps --> Data
    PathOps --> Output
    
    FileOps --> Config
    FileOps --> Data
    FileOps --> Output
    
    TimeOps --> Data
    TimeOps --> Processing
    
    FormatOps --> Data
    FormatOps --> Output
    
    ValidateOps --> Config
    ValidateOps --> Data
    ValidateOps --> Processing
    
    SystemOps --> Processing
    SystemOps --> Error
```

## Utility Components

### Path Utilities

```mermaid
classDiagram
    class PathUtils {
        +normalize_path(path) String
        +join_paths(paths) String
        +get_parent_dir(path) String
        +get_filename(path) String
        +get_extension(path) String
        +is_absolute(path) bool
        +make_relative(base, target) String
        +ensure_directory_exists(path) Result
        +validate_path_safety(path) bool
    }
    
    class PathValidator {
        +is_safe_path(path) bool
        +check_path_traversal(path) bool
        +validate_filename(name) bool
        +check_reserved_names(name) bool
    }
    
    class PathBuilder {
        +new() PathBuilder
        +add_segment(segment) PathBuilder
        +set_extension(ext) PathBuilder
        +build() String
        +build_absolute(base) String
    }
    
    PathUtils --> PathValidator
    PathUtils --> PathBuilder
```

### File Utilities

```mermaid
classDiagram
    class FileUtils {
        +read_file_safe(path) Result~String~
        +write_file_safe(path, content) Result
        +copy_file_safe(src, dst) Result
        +move_file_safe(src, dst) Result
        +delete_file_safe(path) Result
        +get_file_size(path) Result~u64~
        +get_file_metadata(path) Result~Metadata~
        +is_file_readable(path) bool
        +is_file_writable(path) bool
    }
    
    class FileValidator {
        +validate_file_exists(path) bool
        +validate_file_permissions(path) bool
        +validate_file_size(path, max_size) bool
        +validate_file_type(path, allowed_types) bool
    }
    
    class FileMonitor {
        +watch_file(path) FileWatcher
        +watch_directory(path) DirectoryWatcher
        +get_file_changes() Vec~FileEvent~
    }
    
    FileUtils --> FileValidator
    FileUtils --> FileMonitor
```

### Time Utilities

```mermaid
classDiagram
    class TimeUtils {
        +parse_bls_date(date_str) Result~DateTime~
        +format_bls_date(datetime) String
        +parse_period(period_str) Result~Period~
        +format_period(period) String
        +get_current_timestamp() DateTime
        +calculate_duration(start, end) Duration
        +is_valid_year(year) bool
        +is_valid_period(period) bool
    }
    
    class PeriodParser {
        +parse_annual(period) Result~Period~
        +parse_quarterly(period) Result~Period~
        +parse_monthly(period) Result~Period~
        +parse_weekly(period) Result~Period~
    }
    
    class DateValidator {
        +validate_date_range(date, min, max) bool
        +validate_period_format(period) bool
        +validate_year_range(year) bool
    }
    
    TimeUtils --> PeriodParser
    TimeUtils --> DateValidator
```

## Utility Workflows

### File Processing Workflow

```mermaid
flowchart TD
    Start([Start File Operation]) --> ValidatePath{Validate Path}
    ValidatePath -->|Invalid| PathError[Path Validation Error]
    ValidatePath -->|Valid| CheckExists{File Exists?}
    
    CheckExists -->|No| CreateFile[Create File if Needed]
    CheckExists -->|Yes| CheckPermissions{Check Permissions}
    
    CreateFile --> CheckPermissions
    CheckPermissions -->|Insufficient| PermError[Permission Error]
    CheckPermissions -->|Sufficient| PerformOperation[Perform File Operation]
    
    PerformOperation --> ValidateResult{Validate Result}
    ValidateResult -->|Invalid| OpError[Operation Error]
    ValidateResult -->|Valid| UpdateMetadata[Update File Metadata]
    
    UpdateMetadata --> LogOperation[Log Operation]
    LogOperation --> Success([Operation Successful])
    
    PathError --> End([End with Error])
    PermError --> End
    OpError --> End
```

### Data Validation Workflow

```mermaid
sequenceDiagram
    participant App as Application
    participant Validator as Validation Utils
    participant Rules as Validation Rules
    participant Reporter as Error Reporter
    
    App->>Validator: Validate data
    Validator->>Rules: Load validation rules
    Rules->>Validator: Return rules
    
    loop For Each Data Item
        Validator->>Validator: Apply validation rules
        
        alt Validation Passes
            Validator->>App: Continue processing
        else Validation Fails
            Validator->>Reporter: Report validation error
            Reporter->>App: Return error details
        end
    end
    
    Validator->>App: Validation complete
```

### System Resource Monitoring

```mermaid
graph LR
    subgraph "Resource Monitoring"
        MemoryMonitor[Memory Monitor]
        CPUMonitor[CPU Monitor]
        DiskMonitor[Disk Monitor]
        NetworkMonitor[Network Monitor]
    end
    
    subgraph "Metrics Collection"
        MetricsCollector[Metrics Collector]
        MetricsAggregator[Metrics Aggregator]
        MetricsReporter[Metrics Reporter]
    end
    
    subgraph "Alerting System"
        ThresholdChecker[Threshold Checker]
        AlertManager[Alert Manager]
        NotificationSender[Notification Sender]
    end
    
    MemoryMonitor --> MetricsCollector
    CPUMonitor --> MetricsCollector
    DiskMonitor --> MetricsCollector
    NetworkMonitor --> MetricsCollector
    
    MetricsCollector --> MetricsAggregator
    MetricsAggregator --> MetricsReporter
    MetricsAggregator --> ThresholdChecker
    
    ThresholdChecker --> AlertManager
    AlertManager --> NotificationSender
```

## Key Features

### 🛡️ Safety and Security
- **Path Validation**: Comprehensive path safety checks and validation
- **File Permissions**: Proper file permission handling and validation
- **Input Sanitization**: Sanitization of user inputs and file paths
- **Error Handling**: Robust error handling for all utility operations

### ⚡ Performance Optimization
- **Caching**: Intelligent caching of frequently accessed data
- **Lazy Loading**: Lazy initialization of expensive resources
- **Memory Management**: Efficient memory usage patterns
- **Async Operations**: Non-blocking operations where appropriate

### 🔍 Comprehensive Validation
- **Data Validation**: Extensive data validation utilities
- **Format Validation**: Format checking for various data types
- **Range Validation**: Numeric and date range validation
- **Custom Validators**: Support for custom validation rules

### 📊 System Monitoring
- **Resource Monitoring**: Real-time system resource monitoring
- **Performance Metrics**: Collection and reporting of performance metrics
- **Health Checks**: System health monitoring and reporting
- **Alerting**: Configurable alerting for system issues

## Format Utilities

### Data Format Conversion

```mermaid
graph TB
    subgraph "Input Formats"
        StringInput[String Input]
        NumericInput[Numeric Input]
        DateInput[Date Input]
        BooleanInput[Boolean Input]
    end
    
    subgraph "Format Converters"
        StringConverter[String Converter]
        NumericConverter[Numeric Converter]
        DateConverter[Date Converter]
        BooleanConverter[Boolean Converter]
    end
    
    subgraph "Output Formats"
        JSONOutput[JSON Output]
        CSVOutput[CSV Output]
        XMLOutput[XML Output]
        BinaryOutput[Binary Output]
    end
    
    StringInput --> StringConverter
    NumericInput --> NumericConverter
    DateInput --> DateConverter
    BooleanInput --> BooleanConverter
    
    StringConverter --> JSONOutput
    StringConverter --> CSVOutput
    StringConverter --> XMLOutput
    
    NumericConverter --> JSONOutput
    NumericConverter --> CSVOutput
    NumericConverter --> BinaryOutput
    
    DateConverter --> JSONOutput
    DateConverter --> CSVOutput
    DateConverter --> XMLOutput
    
    BooleanConverter --> JSONOutput
    BooleanConverter --> CSVOutput
    BooleanConverter --> BinaryOutput
```

### String Processing Pipeline

```mermaid
flowchart LR
    RawString[Raw String] --> Trim[Trim Whitespace]
    Trim --> Normalize[Normalize Encoding]
    Normalize --> Validate[Validate Format]
    Validate --> Sanitize[Sanitize Content]
    Sanitize --> Transform[Apply Transformations]
    Transform --> ProcessedString[Processed String]
    
    Validate -->|Invalid| ValidationError[Validation Error]
    Sanitize -->|Unsafe| SanitizationError[Sanitization Error]
    Transform -->|Error| TransformError[Transformation Error]
```

## Validation Utilities

### Validation Rule Engine

```mermaid
stateDiagram-v2
    [*] --> LoadRules
    LoadRules --> ParseRules : Rules Loaded
    ParseRules --> ValidateRules : Rules Parsed
    ValidateRules --> ReadyToValidate : Rules Valid
    
    ReadyToValidate --> ApplyRules : Data Received
    ApplyRules --> CheckResult : Rules Applied
    
    CheckResult --> ValidationPassed : All Rules Pass
    CheckResult --> ValidationFailed : Some Rules Fail
    
    ValidationPassed --> [*] : Return Success
    ValidationFailed --> CollectErrors : Collect Error Details
    CollectErrors --> [*] : Return Errors
    
    ParseRules --> RuleError : Parse Error
    ValidateRules --> RuleError : Validation Error
    ApplyRules --> RuleError : Application Error
    
    RuleError --> [*] : Return Rule Error
```

### Data Quality Assessment

```mermaid
graph TB
    subgraph "Quality Dimensions"
        Completeness[Completeness]
        Accuracy[Accuracy]
        Consistency[Consistency]
        Validity[Validity]
        Timeliness[Timeliness]
        Uniqueness[Uniqueness]
    end
    
    subgraph "Quality Metrics"
        CompletenessScore[Completeness Score]
        AccuracyScore[Accuracy Score]
        ConsistencyScore[Consistency Score]
        ValidityScore[Validity Score]
        TimelinessScore[Timeliness Score]
        UniquenessScore[Uniqueness Score]
    end
    
    subgraph "Quality Report"
        OverallScore[Overall Quality Score]
        DetailedReport[Detailed Quality Report]
        Recommendations[Quality Recommendations]
    end
    
    Completeness --> CompletenessScore
    Accuracy --> AccuracyScore
    Consistency --> ConsistencyScore
    Validity --> ValidityScore
    Timeliness --> TimelinessScore
    Uniqueness --> UniquenessScore
    
    CompletenessScore --> OverallScore
    AccuracyScore --> OverallScore
    ConsistencyScore --> OverallScore
    ValidityScore --> OverallScore
    TimelinessScore --> OverallScore
    UniquenessScore --> OverallScore
    
    OverallScore --> DetailedReport
    DetailedReport --> Recommendations
```

## Integration with Other Components

### Configuration System Integration
The Utilities component provides essential services to the Configuration System:
- Path resolution and validation for configuration files
- File reading and writing utilities for configuration persistence
- Validation utilities for configuration data
- Format conversion utilities for different configuration formats

### Data Layer Integration
The Utilities component supports the Data Layer with:
- File handling utilities for data file operations
- Data validation and format conversion utilities
- Time and date processing for BLS data formats
- Path utilities for data file organization

### Processing System Integration
The Utilities component assists the Processing System with:
- System resource monitoring for performance optimization
- Validation utilities for data quality checks
- Format conversion utilities for data transformations
- Time utilities for processing timestamps and durations

### Error Handling Integration
The Utilities component integrates with the Error Handling System:
- Provides detailed error context for utility operations
- Implements error recovery strategies for file operations
- Supports error logging and reporting
- Provides validation error details and suggestions

## Performance Considerations

### Memory Management
- Use memory pools for frequently allocated objects
- Implement lazy loading for expensive operations
- Cache frequently accessed data with appropriate expiration
- Monitor memory usage and optimize allocations

### I/O Optimization
- Use buffered I/O for file operations
- Implement async file operations where beneficial
- Optimize file access patterns
- Use memory mapping for large files when appropriate

### CPU Optimization
- Use efficient algorithms for data processing
- Implement parallel processing for independent operations
- Optimize string processing operations
- Use appropriate data structures for different use cases

## Testing Strategy

### Unit Tests
- Test individual utility functions in isolation
- Test edge cases and error conditions
- Test performance characteristics
- Test security and safety features

### Integration Tests
- Test utility integration with other components
- Test file operations with real file systems
- Test validation with real data
- Test system monitoring with real resources

### Performance Tests
- Benchmark utility function performance
- Test memory usage and optimization
- Test I/O performance and optimization
- Validate caching effectiveness

## Best Practices

### Safety and Security
- Always validate inputs before processing
- Use safe file operations with proper error handling
- Implement proper path validation to prevent traversal attacks
- Sanitize all user inputs and external data

### Performance
- Cache frequently accessed data appropriately
- Use lazy loading for expensive operations
- Optimize critical path operations
- Monitor and profile utility performance

### Error Handling
- Provide detailed error messages with context
- Implement appropriate error recovery strategies
- Log errors with sufficient detail for debugging
- Use proper error types for different error conditions

### Code Quality
- Write clear, self-documenting code
- Use appropriate abstractions and interfaces
- Follow consistent naming conventions
- Implement comprehensive tests

## Troubleshooting

### Common Issues

1. **File Operation Failures**
   - Check file permissions and ownership
   - Verify file paths are correct and accessible
   - Ensure sufficient disk space is available
   - Check for file locking issues

2. **Validation Failures**
   - Review validation rules and criteria
   - Check input data format and structure
   - Verify validation rule configuration
   - Test with known good data

3. **Performance Issues**
   - Profile utility function performance
   - Check for memory leaks or excessive allocations
   - Optimize I/O operations and file access patterns
   - Review caching strategies and effectiveness

4. **Path Resolution Issues**
   - Verify path format and structure
   - Check for path traversal attempts
   - Ensure proper path normalization
   - Test with different operating systems

## Contributing

When contributing to the Utilities component:

1. Follow existing patterns and conventions
2. Implement comprehensive error handling
3. Add appropriate validation and safety checks
4. Write tests for all utility functions
5. Document function behavior and edge cases
6. Consider performance implications
7. Ensure cross-platform compatibility where needed

## Further Reading

- [Path Utilities Reference](path_utils.md)
- [File Utilities Reference](file_utils.md)
- [Time Utilities Reference](time_utils.md)
- [Validation Utilities Reference](validation_utils.md)
- [Performance Optimization Guide](performance.md)

---

For questions about the Utilities component, please refer to the troubleshooting section or create an issue in the project repository.