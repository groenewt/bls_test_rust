# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with the Rust-based BLS data processing system.

## PROJECT OVERVIEW

A high-performance, parallel data processing system for 60+ Bureau of Labor Statistics (BLS) surveys, handling datasets from kilobytes to 5+ gigabytes with zero-copy streaming and intelligent partitioning.

**Core Technologies:**
- **Rust** - Systems programming with memory safety
- **Tokio** - Async runtime for concurrent I/O operations
- **Arrow** - Columnar memory format for analytics
- **Parquet** - Efficient columnar storage format
- **Rayon** - Data parallelism for CPU-bound operations

**Key Features:**
- YAML-driven configuration for 60+ different survey formats
- Adaptive partitioning for files up to 5+ GB
- Zero-copy processing with memory-mapped files
- Streaming transformations with minimal memory footprint
- Environment-variable controlled parallelization

## ARCHITECTURE COMPONENTS

### 1. CONFIGURATION LAYER (`src/config/`)
```rust
// Survey configuration loaded from YAML
pub struct SurveyConfig {
    main: MainConfig,           // Survey metadata
    processing: ProcessingConfig, // Processing parameters
    file_patterns: FilePatterns, // File discovery patterns
    data_structures: HashMap<String, TableSchema>, // Schema definitions
    transformations: Vec<TransformRule>, // Data transformations
    output: OutputConfig,        // Output formatting
}
```

### 2. DISCOVERY ENGINE (`src/discovery/`)
```rust
// Intelligent file discovery and analysis
pub struct FileDiscovery {
    patterns: GlobPatterns,
    validator: SchemaValidator,
    size_analyzer: SizeAnalyzer,
}
```

### 3. PARTITION PLANNER (`src/partition/`)
```rust
// Adaptive partitioning based on file characteristics
pub enum PartitionStrategy {
    BySize { target_mb: usize },     // Size-based splitting
    ByColumn { column: String },     // Column-value partitioning
    ByTimeRange { interval: Duration }, // Temporal partitioning
    Adaptive,                        // Auto-select strategy
}
```

### 4. PROCESSING ENGINE (`src/processing/`)
```rust
// Core processing pipeline
pub struct ProcessingEngine {
    memory_manager: MemoryManager,   // Memory pool management
    stream_processor: StreamProcessor, // Streaming data processor
    transform_engine: TransformEngine, // Data transformations
    metrics: Arc<Metrics>,           // Performance monitoring
}
```

### 5. OUTPUT WRITER (`src/output/`)
```rust
// High-performance Parquet writer
pub struct ParquetWriter {
    writer_props: WriterProperties, // Compression, encoding settings
    partitioner: OutputPartitioner, // Output partitioning logic
    statistics: Statistics,         // Column statistics
}
```

## ENVIRONMENT VARIABLES

The system respects these environment variables for runtime configuration:

```bash
# Parallelization Control
BLS_MAX_THREADS=16              # Max parallel threads (default: num_cpus)
BLS_TOKIO_THREADS=4             # Async runtime threads
BLS_RAYON_THREADS=8             # Data parallelism threads

# Memory Management
BLS_MEMORY_LIMIT_GB=8           # Max memory usage in GB
BLS_CHUNK_SIZE_MB=256           # Processing chunk size
BLS_BUFFER_POOL_SIZE=8          # Number of pre-allocated buffers
BLS_ENABLE_MMAP=true            # Use memory-mapped files

# Partitioning
BLS_PARTITION_THRESHOLD_GB=1.0  # File size triggering partitioning
BLS_PARTITION_STRATEGY=adaptive # Default partition strategy
BLS_MAX_PARTITIONS=100          # Maximum partition count

# Output Configuration
BLS_OUTPUT_FORMAT=parquet        # Output format (parquet/csv)
BLS_COMPRESSION=snappy           # Compression (snappy/gzip/zstd/lz4)
BLS_ROW_GROUP_SIZE=100000       # Parquet row group size
BLS_ENABLE_STATISTICS=true      # Column statistics
BLS_ENABLE_BLOOM_FILTERS=false  # Bloom filters for columns

# Monitoring
BLS_LOG_LEVEL=info              # Log level (trace/debug/info/warn/error)
BLS_METRICS_INTERVAL_SEC=10     # Metrics reporting interval
BLS_ENABLE_PROFILING=false      # CPU/memory profiling
```

## PROCESSING WORKFLOW

### STAGE 1: INITIALIZATION
```bash
# Load survey configuration
cargo run -- init --survey si --config config/si.yml

# Validates:
- YAML configuration syntax
- Schema definitions
- File pattern matching
- Output directory permissions
```

### STAGE 2: DISCOVERY & PLANNING
```bash
# Discover and analyze input files
cargo run -- discover --survey si --input data/raw/bls/si/

# Performs:
- File pattern matching
- Size analysis (identifies 5GB+ files)
- Schema validation
- Partition planning
```

### STAGE 3: SERIES PROCESSING
```bash
# Process series definition file
cargo run -- process-series --survey si --series data/raw/bls/si/si.series

# Creates:
- Series index for lookups
- Column mappings
- Validation rules
```

### STAGE 4: DATA PROCESSING
```bash
# Process data files with parallelization
cargo run -- process-data --survey si --input data/raw/bls/si/ --output data/processed/si/

# Executes:
- Parallel partition processing
- Streaming transformations
- Memory-controlled operations
- Progress monitoring
```

### STAGE 5: OUTPUT GENERATION
```bash
# Generate final Parquet output
cargo run -- export --survey si --format parquet --output data/final/si/

# Produces:
- Partitioned Parquet files
- Column statistics
- Metadata catalog
```

## MEMORY MANAGEMENT STRATEGIES

### For Small Files (<100MB)
- Load entirely into Arrow arrays
- Process in single batch
- Direct Parquet write

### For Medium Files (100MB - 1GB)
- Stream processing with 256MB chunks
- Parallel transformation pipeline
- Buffered Parquet writing

### For Large Files (1GB - 5GB)
- Memory-mapped file access
- Adaptive partitioning
- Concurrent partition processing
- Streaming aggregations

### For Massive Files (5GB+)
- Aggressive partitioning
- Distributed processing across threads
- Minimal memory footprint per partition
- Checkpoint-based recovery

## ERROR HANDLING

The system implements comprehensive error handling:

```rust
// All operations return Result<T, BLSError>
match process_survey(&config).await {
    Ok(metrics) => {
        info!("Processed {} records in {:?}", 
              metrics.records_processed, 
              metrics.elapsed);
    }
    Err(BLSError::MemoryExceeded { used_mb, limit_mb }) => {
        // Automatic retry with smaller chunks
        warn!("Memory limit exceeded, retrying with smaller chunks");
        config.processing.chunk_size_mb /= 2;
        process_survey(&config).await?;
    }
    Err(e) => {
        error!("Processing failed: {}", e);
        // Checkpoint recovery for resumable processing
        recover_from_checkpoint(&config).await?;
    }
}
```

## PERFORMANCE MONITORING

Real-time metrics available via:

```rust
// Metrics structure updated during processing
pub struct Metrics {
    start_time: Instant,
    records_processed: AtomicU64,
    bytes_processed: AtomicU64,
    current_memory_mb: AtomicUsize,
    peak_memory_mb: AtomicUsize,
    partitions_completed: AtomicU32,
    errors_recovered: AtomicU32,
}

// Access metrics during processing
let metrics = engine.metrics();
info!("Throughput: {:.2} MB/s", metrics.throughput_mbps());
info!("Memory usage: {} MB", metrics.current_memory_mb());
```

## CONFIGURATION EXAMPLES

### SI Survey Configuration
```yaml
main:
  code: si
  name: Occupation Injuries and Illness Incidences Rate Data
  
processing:
  partition_strategy: adaptive
  compression: snappy
  max_row_group_size: 100000
  
file_patterns:
  series: "si.series"
  data: "si.data.*"
  lookups:
    - "si.case_type"
    - "si.division"
    - "si.industry"

transformations:
  - source: series_id
    target: series_id
    type: direct
  - source: division_code
    target: division_name
    type: lookup
    table: division
  - source: value
    target: value_normalized
    type: expression
    expression: "value * 100.0 / base_value"

output:
  format: parquet
  partition_by: [year, division_code]
  compression: snappy
  enable_statistics: true
```

## DEVELOPMENT WORKFLOW

### 1. Adding a New Survey
```bash
# Create configuration
cp config/template.yml config/new_survey.yml
# Edit configuration with survey-specific details

# Test configuration
cargo test --test config_validation -- --survey new_survey

# Process sample data
cargo run -- process --survey new_survey --sample --limit 1000
```

### 2. Optimizing Performance
```bash
# Enable profiling
export BLS_ENABLE_PROFILING=true
cargo run --release -- process --survey si

# Analyze flamegraph
cargo flamegraph -- process --survey si

# Memory profiling
valgrind --tool=massif target/release/bls_processor
```

### 3. Testing Large Files
```bash
# Generate test file
cargo run --bin generate_test_data -- --size 5gb --format si

# Process with monitoring
BLS_LOG_LEVEL=debug BLS_METRICS_INTERVAL_SEC=1 \
cargo run --release -- process --survey si --input test_data/
```

## TROUBLESHOOTING

### Memory Issues
```bash
# Reduce memory usage
export BLS_MEMORY_LIMIT_GB=4
export BLS_CHUNK_SIZE_MB=128
export BLS_BUFFER_POOL_SIZE=4
```

### Slow Processing
```bash
# Increase parallelization
export BLS_MAX_THREADS=32
export BLS_RAYON_THREADS=16
export BLS_PARTITION_THRESHOLD_GB=0.5  # More aggressive partitioning
```

### File Too Large
```bash
# Force partitioning
export BLS_PARTITION_STRATEGY=by_size
export BLS_PARTITION_THRESHOLD_GB=0.1
export BLS_MAX_PARTITIONS=1000
```

## BEST PRACTICES

1. **Always validate configurations** before processing
2. **Monitor memory usage** for datasets >1GB
3. **Use adaptive partitioning** for unknown file sizes
4. **Enable statistics** for Parquet files (query optimization)
5. **Set appropriate row group sizes** (100K-1M rows)
6. **Use compression** (snappy for speed, zstd for size)
7. **Implement checkpointing** for long-running processes
8. **Profile before optimizing** specific surveys

## COMMON PATTERNS

### Processing Multiple Surveys
```rust
let surveys = vec!["si", "sm", "tu", "ap"];
let handles: Vec<_> = surveys
    .into_iter()
    .map(|survey| {
        tokio::spawn(async move {
            process_survey(survey).await
        })
    })
    .collect();

let results = futures::future::try_join_all(handles).await?;
```

### Custom Transformations
```rust
// Register custom transformation
engine.register_transform("normalize_rate", |value: f64| {
    value * 100.0 / 40.0  // Per 100 full-time workers
});
```

### Incremental Processing
```rust
// Process only new files
let processed_files = load_manifest("manifest.json")?;
let new_files = discovered_files
    .into_iter()
    .filter(|f| !processed_files.contains(f))
    .collect();

process_files(new_files).await?;
```