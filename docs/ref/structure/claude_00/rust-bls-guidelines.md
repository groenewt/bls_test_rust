# RUST BLS PROCESSING SYSTEM GUIDELINES

## CORE ARCHITECTURE PRINCIPLES

### 1. ZERO-COPY PROCESSING
- Use memory-mapped files (`memmap2`) for large datasets
- Stream processing with async iterators
- Arrow columnar format for in-memory operations
- Direct Parquet writing without intermediate formats

### 2. PARALLELIZATION STRATEGY

```rust
// Environment-driven configuration
pub struct ParallelConfig {
    // From ENV vars
    pub max_threads: usize,           // BLS_MAX_THREADS (default: num_cpus)
    pub chunk_size_mb: usize,         // BLS_CHUNK_SIZE_MB (default: 256)
    pub buffer_pool_size: usize,      // BLS_BUFFER_POOL_SIZE (default: 8)
    pub enable_mmap: bool,            // BLS_ENABLE_MMAP (default: true)
    pub partition_threshold_gb: f64,  // BLS_PARTITION_THRESHOLD_GB (default: 1.0)
}

// Smart partitioning based on file size
pub enum PartitionStrategy {
    BySize { target_mb: usize },
    ByColumn { column: String, max_values: usize },
    ByTimeRange { interval: ChronoInterval },
    ByHash { buckets: usize },
    Adaptive, // Automatically choose based on data characteristics
}
```

### 3. MEMORY MANAGEMENT ARCHITECTURE

```rust
use arrow::array::ArrayRef;
use arrow::record_batch::RecordBatch;

pub struct MemoryManager {
    // Pre-allocated buffer pools
    buffer_pools: Vec<ByteBuffer>,
    
    // Memory-mapped regions for large files
    mmap_regions: HashMap<PathBuf, Mmap>,
    
    // Arrow memory pool for columnar operations
    arrow_pool: Arc<MemoryPool>,
    
    // Adaptive memory limits
    memory_limit_bytes: usize,
    current_usage: AtomicUsize,
}

// Streaming processor for massive files
pub struct StreamProcessor<R: Read + Seek> {
    reader: BufReader<R>,
    schema: Arc<Schema>,
    batch_size: usize,
    decoder: Box<dyn RecordDecoder>,
}
```

### 4. YAML-DRIVEN CONFIGURATION

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct SurveyConfig {
    pub main: MainConfig,
    pub processing: ProcessingConfig,
    pub file_patterns: FilePatterns,
    pub data_structures: HashMap<String, TableSchema>,
    pub transformations: Vec<TransformRule>,
    pub output: OutputConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessingConfig {
    pub partition_strategy: PartitionStrategy,
    pub compression: CompressionType,
    pub enable_statistics: bool,
    pub max_row_group_size: usize,
    pub enable_bloom_filters: bool,
    pub dictionary_encoding_threshold: f32,
}
```

## PROCESSING PIPELINE

### STAGE 1: CONFIGURATION & DISCOVERY

```rust
pub async fn initialize_survey(survey_code: &str) -> Result<SurveyPipeline> {
    // 1. Load YAML configuration
    let config = load_config(&format!("config/{}.yml", survey_code)).await?;
    
    // 2. Validate configuration against data files
    let validator = ConfigValidator::new(&config);
    validator.validate_schema().await?;
    
    // 3. Discover files matching patterns
    let file_discovery = FileDiscovery::new(&config.file_patterns);
    let files = file_discovery.discover_all().await?;
    
    // 4. Analyze file sizes and determine partition strategy
    let strategy = PartitionPlanner::analyze(&files, &config).await?;
    
    Ok(SurveyPipeline {
        config,
        files,
        strategy,
        metrics: Arc::new(Metrics::new()),
    })
}
```

### STAGE 2: SERIES FILE PROCESSING

```rust
pub async fn process_series_file(
    series_path: &Path,
    schema: &TableSchema,
) -> Result<SeriesIndex> {
    // Memory-map for efficient access
    let file = File::open(series_path).await?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    
    // Parse series definitions
    let mut series_index = SeriesIndex::new();
    let parser = SeriesParser::new(schema);
    
    // Stream parse with async chunks
    let mut stream = parser.parse_stream(&mmap);
    while let Some(batch) = stream.next().await {
        let batch = batch?;
        series_index.add_batch(batch)?;
    }
    
    // Build indexes for fast lookups
    series_index.build_indexes()?;
    Ok(series_index)
}
```

### STAGE 3: DATA FILE PROCESSING

```rust
pub async fn process_data_files(
    pipeline: &SurveyPipeline,
    series_index: &SeriesIndex,
) -> Result<ProcessedDataset> {
    let semaphore = Arc::new(Semaphore::new(pipeline.config.processing.max_threads));
    let mut tasks = Vec::new();
    
    for (file_path, partition_plan) in &pipeline.partitions {
        let sem = semaphore.clone();
        let series_idx = series_index.clone();
        let config = pipeline.config.clone();
        
        let task = tokio::spawn(async move {
            let _permit = sem.acquire().await?;
            process_partition(file_path, partition_plan, &series_idx, &config).await
        });
        
        tasks.push(task);
    }
    
    // Collect results with progress tracking
    let results = futures::future::try_join_all(tasks).await?;
    Ok(ProcessedDataset::merge(results))
}

async fn process_partition(
    file_path: &Path,
    plan: &PartitionPlan,
    series_index: &SeriesIndex,
    config: &SurveyConfig,
) -> Result<PartitionResult> {
    match plan.strategy {
        PartitionStrategy::BySize { target_mb } => {
            // Split file into chunks
            let chunks = split_by_size(file_path, target_mb).await?;
            process_chunks_parallel(chunks, series_index, config).await
        }
        PartitionStrategy::Adaptive => {
            // Analyze first 1MB to determine best strategy
            let sample = read_sample(file_path, 1024 * 1024).await?;
            let optimal_strategy = analyze_optimal_strategy(&sample)?;
            process_with_strategy(file_path, optimal_strategy, series_index, config).await
        }
        _ => process_single_partition(file_path, series_index, config).await,
    }
}
```

### STAGE 4: TRANSFORMATION ENGINE

```rust
pub struct TransformationEngine {
    rules: Vec<TransformRule>,
    lookup_tables: HashMap<String, LookupTable>,
    expression_engine: ExpressionEngine,
}

impl TransformationEngine {
    pub async fn transform_batch(
        &self,
        batch: RecordBatch,
    ) -> Result<RecordBatch> {
        let mut arrays: Vec<ArrayRef> = Vec::new();
        let mut fields: Vec<Field> = Vec::new();
        
        for rule in &self.rules {
            let array = match &rule.transform_type {
                TransformType::Direct => {
                    batch.column_by_name(&rule.source_field).cloned()
                }
                TransformType::Lookup { table, key_field } => {
                    self.apply_lookup(&batch, table, key_field).await?
                }
                TransformType::Expression(expr) => {
                    self.expression_engine.evaluate(expr, &batch)?
                }
                TransformType::Aggregate(agg) => {
                    self.apply_aggregation(agg, &batch).await?
                }
            }?;
            
            arrays.push(array);
            fields.push(Field::new(&rule.target_field, rule.data_type.clone(), rule.nullable));
        }
        
        Ok(RecordBatch::try_new(
            Arc::new(Schema::new(fields)),
            arrays,
        )?)
    }
}
```

### STAGE 5: OUTPUT GENERATION

```rust
pub async fn write_parquet_output(
    dataset: ProcessedDataset,
    output_config: &OutputConfig,
) -> Result<OutputMetadata> {
    let writer_props = WriterProperties::builder()
        .set_compression(output_config.compression)
        .set_statistics_enabled(output_config.enable_statistics)
        .set_max_row_group_size(output_config.max_row_group_size)
        .set_bloom_filter_enabled(output_config.enable_bloom_filters)
        .set_dictionary_enabled(output_config.dictionary_encoding)
        .build();
    
    // Partition output by configured strategy
    let partitions = match &output_config.partition_by {
        Some(columns) => dataset.partition_by(columns)?,
        None => vec![dataset],
    };
    
    let mut output_files = Vec::new();
    
    for (partition_key, partition_data) in partitions {
        let output_path = build_output_path(&output_config.base_path, &partition_key);
        
        // Stream write with memory control
        let file = File::create(&output_path).await?;
        let mut writer = ArrowWriter::try_new(file, partition_data.schema(), Some(writer_props))?;
        
        // Write in controlled batches
        for batch in partition_data.batches() {
            writer.write(&batch)?;
        }
        
        let metadata = writer.close()?;
        output_files.push(OutputFile {
            path: output_path,
            partition_key,
            row_count: metadata.num_rows,
            size_bytes: metadata.serialized_size,
        });
    }
    
    Ok(OutputMetadata {
        files: output_files,
        total_rows: dataset.row_count(),
        processing_time: dataset.metrics.elapsed(),
    })
}
```

## PERFORMANCE OPTIMIZATIONS

### 1. SIMD OPERATIONS
```rust
// Use arrow's SIMD-optimized operations
use arrow::compute::{cast, filter, take};

pub fn optimize_filtering(batch: &RecordBatch, predicate: &BooleanArray) -> Result<RecordBatch> {
    // SIMD-accelerated filtering
    let indices = filter::filter_record_batch(batch, predicate)?;
    take::take_record_batch(batch, &indices)
}
```

### 2. MEMORY POOLING
```rust
pub struct BufferPool {
    pools: Vec<Vec<BytesMut>>,
    size_classes: Vec<usize>,
}

impl BufferPool {
    pub fn acquire(&mut self, size: usize) -> BytesMut {
        let class_idx = self.size_class_for(size);
        if let Some(buffer) = self.pools[class_idx].pop() {
            buffer
        } else {
            BytesMut::with_capacity(self.size_classes[class_idx])
        }
    }
    
    pub fn release(&mut self, mut buffer: BytesMut) {
        buffer.clear();
        let class_idx = self.size_class_for(buffer.capacity());
        self.pools[class_idx].push(buffer);
    }
}
```

### 3. ADAPTIVE BATCH SIZING
```rust
pub struct AdaptiveBatcher {
    target_memory_mb: usize,
    min_batch_size: usize,
    max_batch_size: usize,
    current_batch_size: usize,
    performance_history: VecDeque<BatchMetrics>,
}

impl AdaptiveBatcher {
    pub fn next_batch_size(&mut self, last_metrics: BatchMetrics) -> usize {
        self.performance_history.push_back(last_metrics);
        if self.performance_history.len() > 10 {
            self.performance_history.pop_front();
        }
        
        // Adjust based on memory pressure and throughput
        let avg_memory = self.avg_memory_usage();
        let avg_throughput = self.avg_throughput();
        
        if avg_memory > self.target_memory_mb as f64 * 0.9 {
            self.current_batch_size = (self.current_batch_size * 0.8) as usize;
        } else if avg_memory < self.target_memory_mb as f64 * 0.5 {
            self.current_batch_size = (self.current_batch_size * 1.2) as usize;
        }
        
        self.current_batch_size.clamp(self.min_batch_size, self.max_batch_size)
    }
}
```

## ERROR HANDLING & RECOVERY

```rust
#[derive(Debug, thiserror::Error)]
pub enum BLSError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("File processing error in {file}: {message}")]
    FileProcessing { file: PathBuf, message: String },
    
    #[error("Memory limit exceeded: {used_mb}MB > {limit_mb}MB")]
    MemoryExceeded { used_mb: usize, limit_mb: usize },
    
    #[error("Transformation error: {0}")]
    Transformation(String),
    
    #[error("Partition error: {0}")]
    Partition(String),
}

pub struct RecoveryStrategy {
    pub checkpoint_interval: Duration,
    pub max_retries: usize,
    pub fallback_partition_size: usize,
}

impl RecoveryStrategy {
    pub async fn with_recovery<F, T>(
        &self,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> Future<Output = Result<T>> + Clone,
    {
        let mut retries = 0;
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if retries < self.max_retries => {
                    warn!("Operation failed, retry {}/{}: {}", retries + 1, self.max_retries, e);
                    retries += 1;
                    tokio::time::sleep(Duration::from_secs(2_u64.pow(retries))).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

## MONITORING & METRICS

```rust
pub struct ProcessingMetrics {
    pub start_time: Instant,
    pub records_processed: AtomicU64,
    pub bytes_processed: AtomicU64,
    pub memory_peak_bytes: AtomicUsize,
    pub partition_metrics: DashMap<String, PartitionMetrics>,
}

impl ProcessingMetrics {
    pub fn record_batch_processed(&self, batch: &RecordBatch) {
        self.records_processed.fetch_add(batch.num_rows() as u64, Ordering::Relaxed);
        let bytes = batch.get_array_memory_size();
        self.bytes_processed.fetch_add(bytes as u64, Ordering::Relaxed);
    }
    
    pub fn throughput_mbps(&self) -> f64 {
        let bytes = self.bytes_processed.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();
        (bytes as f64 / 1_000_000.0) / elapsed
    }
}
```

## TESTING STRATEGY

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_partition_strategy(
            file_size in 1_000_000..10_000_000_000_u64,
            target_partition_size in 100_000_000..1_000_000_000_u64,
        ) {
            let strategy = PartitionPlanner::determine_strategy(file_size, target_partition_size);
            let partitions = strategy.plan_partitions(file_size);
            
            // Verify no partition exceeds target size
            for partition in &partitions {
                assert!(partition.estimated_size <= target_partition_size);
            }
            
            // Verify complete coverage
            let total_size: u64 = partitions.iter().map(|p| p.estimated_size).sum();
            assert_eq!(total_size, file_size);
        }
    }
}
```