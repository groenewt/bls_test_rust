# BLS DATA PROCESSING SYSTEM - TECHNICAL GUIDELINES v2.0

## SYSTEM OVERVIEW

A high-performance, configuration-driven system for processing 60+ BLS surveys with datasets ranging from megabytes to 5GB+. Built for parallel processing, intelligent partitioning, and memory-efficient operations.

## CORE ARCHITECTURE PRINCIPLES

### 1. THREE-PHASE PROCESSING MODEL

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   DISCOVERY     │────▶│    STAGING      │────▶│    ASSEMBLY     │
│                 │     │                 │     │                 │
│ • Config Load   │     │ • Series Parse  │     │ • Partition Join│
│ • File Scan     │     │ • Data Chunk    │     │ • Format Export │
│ • Size Analysis │     │ • Transform     │     │ • Validation    │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

### 2. INTELLIGENT PARTITIONING SYSTEM

**MANDATORY: Dynamic partition strategy based on data characteristics**

```c
typedef enum {
    PARTITION_BY_SIZE,        // Split when file > threshold
    PARTITION_BY_SERIES,      // Group by series prefix
    PARTITION_BY_TIME,        // Split by year/period
    PARTITION_BY_GEOGRAPHY,   // Split by state/area
    PARTITION_BY_INDUSTRY,    // Split by industry code
    PARTITION_HYBRID          // Multi-dimensional partitioning
} PartitionStrategy;

typedef struct {
    PartitionStrategy strategy;
    size_t max_partition_size_mb;
    size_t target_partitions;
    char partition_column[64];
    bool enable_compression;
    int compression_level;
} PartitionConfig;
```

### 3. ENVIRONMENT-DRIVEN CONFIGURATION

**MANDATORY: All runtime parameters via environment variables**

```bash
# Core Processing Settings
export BLS_MAX_WORKERS=16                    # Parallel worker threads
export BLS_MEMORY_LIMIT_GB=8                 # Total memory limit
export BLS_PARTITION_SIZE_MB=256             # Target partition size
export BLS_STAGING_DIR=/tmp/bls/staging      # Staging directory
export BLS_OUTPUT_FORMAT=parquet             # Output format (parquet|csv)

# Performance Tuning
export BLS_BUFFER_SIZE_MB=64                 # I/O buffer size
export BLS_BATCH_SIZE=10000                  # Records per batch
export BLS_COMPRESSION=snappy                # Compression algorithm
export BLS_MEMORY_MAP_THRESHOLD_MB=512       # When to use mmap

# Processing Behavior
export BLS_FAIL_FAST=false                   # Stop on first error
export BLS_VALIDATE_OUTPUT=true              # Validate final output
export BLS_KEEP_STAGING=false                # Retain staging files
export BLS_PROFILE_PERFORMANCE=true          # Enable profiling
```

## IMPLEMENTATION ARCHITECTURE

### 1. SURVEY ORCHESTRATOR

**MANDATORY: Central coordinator for all survey processing**

```c
typedef struct {
    char survey_code[8];
    SurveyConfig* config;
    PartitionStrategy partition_strategy;
    SeriesIndex* series_index;
    DataPartition** partitions;
    size_t partition_count;
    ProcessingStats stats;
} SurveyOrchestrator;

typedef struct {
    // Series indexing for fast lookups
    GHashTable* series_map;        // series_id -> metadata
    BTree* time_index;             // Indexed by year/period
    BTree* geo_index;              // Indexed by geography
    size_t total_series;
    size_t memory_footprint;
} SeriesIndex;

typedef struct {
    char partition_id[64];
    char** input_files;
    size_t file_count;
    size_t estimated_size;
    size_t record_count;
    PartitionBounds bounds;        // Data boundaries
    ProcessingState state;
} DataPartition;
```

### 2. PARALLEL PROCESSING ENGINE

**MANDATORY: Work-stealing thread pool with dynamic load balancing**

```c
typedef struct {
    pthread_t* workers;
    size_t worker_count;
    WorkQueue* global_queue;
    WorkQueue** local_queues;     // Per-thread work queues
    atomic_bool shutdown;
    pthread_mutex_t coordinator_lock;
    pthread_cond_t work_available;
} ParallelEngine;

typedef struct {
    enum { PARSE_SERIES, PROCESS_PARTITION, MERGE_RESULTS } task_type;
    void* task_data;
    size_t priority;               // Higher priority = process first
    size_t estimated_cost;         // Computational cost estimate
    TaskCallback callback;
    void* callback_data;
} WorkItem;

// Work-stealing algorithm
WorkItem* steal_work(ParallelEngine* engine, size_t worker_id) {
    // Try local queue first
    WorkItem* item = dequeue_local(engine->local_queues[worker_id]);
    if (item) return item;
    
    // Try global queue
    item = dequeue_global(engine->global_queue);
    if (item) return item;
    
    // Steal from other workers
    for (size_t i = 0; i < engine->worker_count; i++) {
        if (i != worker_id) {
            item = steal_from_queue(engine->local_queues[i]);
            if (item) return item;
        }
    }
    return NULL;
}
```

### 3. MEMORY MANAGEMENT V2

**MANDATORY: Arena allocators with partition-specific memory pools**

```c
typedef struct {
    void* base_address;
    size_t total_size;
    size_t used_size;
    size_t chunk_size;
    MemoryChunk* free_list;
    pthread_spinlock_t lock;      // Fast spinlock for allocation
} ArenaAllocator;

typedef struct {
    ArenaAllocator* metadata_arena;    // For series/lookup data
    ArenaAllocator* partition_arena;   // For partition processing
    ArenaAllocator* output_arena;      // For output formatting
    size_t total_allocated;
    size_t peak_usage;
    atomic_size_t current_usage;
} MemoryManager;

// Per-partition memory context
typedef struct {
    ArenaAllocator* arena;
    size_t allocation_limit;
    size_t current_usage;
    bool use_mmap;                     // Use memory-mapped files
    int mmap_fd;                       // File descriptor for mmap
} PartitionMemory;
```

### 4. STAGING SYSTEM

**MANDATORY: Efficient intermediate data management**

```c
typedef struct {
    char staging_root[PATH_MAX];
    char survey_dir[PATH_MAX];
    HashTable* file_registry;          // Track all staging files
    size_t total_staged_size;
    bool compression_enabled;
} StagingManager;

typedef struct {
    char file_path[PATH_MAX];
    char partition_id[64];
    enum { SERIES_INDEX, DATA_CHUNK, LOOKUP_TABLE } file_type;
    size_t file_size;
    time_t creation_time;
    bool is_compressed;
    char checksum[65];                 // SHA-256 checksum
} StagedFile;

// Staging operations
typedef struct {
    FILE* (*open_staged)(const char* partition_id, const char* suffix);
    bool (*write_batch)(FILE* f, const RecordBatch* batch);
    bool (*flush_staged)(FILE* f);
    StagedFile* (*register_staged)(const char* path, const char* partition_id);
    bool (*cleanup_staged)(const char* partition_id);
} StagingOps;
```

### 5. OUTPUT ASSEMBLY

**MANDATORY: Efficient final output generation**

```c
typedef struct {
    enum { CSV, PARQUET } format;
    CompressionType compression;
    PartitionScheme partition_scheme;
    ValidationRules* validation;
    OutputMetadata* metadata;
} OutputConfig;

typedef struct {
    char base_path[PATH_MAX];
    char** partition_paths;
    size_t partition_count;
    size_t total_records;
    size_t total_size;
    time_t creation_time;
    char schema_version[32];
} OutputManifest;

// Parquet-specific configuration
typedef struct {
    size_t row_group_size;
    CompressionCodec codec;
    int compression_level;
    bool enable_dictionary;
    bool enable_statistics;
    size_t page_size;
} ParquetConfig;
```

## PROCESSING WORKFLOW

### PHASE 1: DISCOVERY & ANALYSIS

```c
typedef struct {
    SurveyConfig* config;
    FileInventory* inventory;
    DataProfile* profile;
    PartitionPlan* plan;
} DiscoveryResult;

DiscoveryResult* discover_survey(const char* survey_code) {
    // 1. Load YAML configuration
    SurveyConfig* config = load_yaml_config(survey_code);
    
    // 2. Scan for data files
    FileInventory* inventory = scan_data_files(config->file_patterns);
    
    // 3. Profile data characteristics
    DataProfile* profile = profile_data(inventory);
    
    // 4. Generate partition plan
    PartitionPlan* plan = create_partition_plan(profile, get_env_settings());
    
    return create_discovery_result(config, inventory, profile, plan);
}
```

### PHASE 2: STAGING & TRANSFORMATION

```c
typedef struct {
    SeriesIndex* series_index;
    PartitionResult** partition_results;
    size_t partition_count;
    ProcessingStats stats;
} StagingResult;

StagingResult* stage_survey(DiscoveryResult* discovery) {
    // 1. Parse and index series file
    SeriesIndex* series = parse_series_file(discovery->config);
    
    // 2. Create parallel engine
    ParallelEngine* engine = create_parallel_engine(get_worker_count());
    
    // 3. Process partitions in parallel
    for (size_t i = 0; i < discovery->plan->partition_count; i++) {
        WorkItem* work = create_partition_work(discovery->plan->partitions[i]);
        submit_work(engine, work);
    }
    
    // 4. Wait for completion
    PartitionResult** results = wait_all_partitions(engine);
    
    return create_staging_result(series, results);
}
```

### PHASE 3: ASSEMBLY & OUTPUT

```c
typedef struct {
    OutputManifest* manifest;
    ValidationReport* validation;
    PerformanceMetrics* metrics;
} AssemblyResult;

AssemblyResult* assemble_output(StagingResult* staging, OutputConfig* config) {
    // 1. Create output structure
    OutputBuilder* builder = create_output_builder(config);
    
    // 2. Merge partitions
    for (size_t i = 0; i < staging->partition_count; i++) {
        merge_partition(builder, staging->partition_results[i]);
    }
    
    // 3. Write final output
    OutputManifest* manifest = write_output(builder);
    
    // 4. Validate if required
    ValidationReport* validation = validate_output(manifest, staging->series_index);
    
    return create_assembly_result(manifest, validation);
}
```

## OPTIMIZATION STRATEGIES

### 1. SMART PARTITIONING

```c
PartitionPlan* optimize_partitions(DataProfile* profile) {
    // Analyze data distribution
    if (profile->total_size_gb > 1.0) {
        if (profile->has_time_dimension) {
            return partition_by_year_and_series(profile);
        } else if (profile->has_geography) {
            return partition_by_state(profile);
        } else {
            return partition_by_size(profile, 256 * MB);
        }
    }
    return single_partition(profile);
}
```

### 2. MEMORY PRESSURE MANAGEMENT

```c
void handle_memory_pressure(MemoryManager* mm) {
    size_t available = get_available_memory();
    size_t threshold = get_env_memory_limit() * 0.8;
    
    if (mm->current_usage > threshold) {
        // Flush staged data to disk
        flush_all_staging();
        
        // Compact arenas
        compact_arenas(mm);
        
        // Reduce batch sizes
        reduce_batch_sizes();
    }
}
```

### 3. I/O OPTIMIZATION

```c
typedef struct {
    int read_ahead_kb;
    bool use_direct_io;
    bool use_mmap_for_large;
    size_t io_buffer_size;
    int prefetch_distance;
} IOConfig;

// Adaptive I/O based on file size
IOStrategy select_io_strategy(size_t file_size) {
    if (file_size > 512 * MB) {
        return (IOStrategy) {
            .method = IO_MMAP,
            .prefetch = true,
            .buffer_size = 0  // Not needed for mmap
        };
    } else if (file_size > 64 * MB) {
        return (IOStrategy) {
            .method = IO_BUFFERED,
            .prefetch = true,
            .buffer_size = 8 * MB
        };
    } else {
        return (IOStrategy) {
            .method = IO_STANDARD,
            .prefetch = false,
            .buffer_size = 64 * KB
        };
    }
}
```

## ERROR RECOVERY

### 1. CHECKPOINT SYSTEM

```c
typedef struct {
    char checkpoint_dir[PATH_MAX];
    time_t last_checkpoint;
    PartitionState* partition_states;
    size_t state_count;
} CheckpointManager;

// Periodic checkpointing
void checkpoint_progress(CheckpointManager* cm, const char* partition_id) {
    if (time(NULL) - cm->last_checkpoint > 60) {  // Every minute
        save_partition_state(cm, partition_id);
        cm->last_checkpoint = time(NULL);
    }
}

// Recovery from checkpoint
bool recover_from_checkpoint(const char* survey_code) {
    CheckpointManager* cm = load_checkpoints(survey_code);
    if (cm && cm->state_count > 0) {
        log_info("Recovering from %zu partition checkpoints", cm->state_count);
        return resume_processing(cm);
    }
    return false;
}
```

### 2. PARTIAL FAILURE HANDLING

```c
typedef enum {
    RETRY_WITH_BACKOFF,
    SKIP_PARTITION,
    FAIL_FAST,
    QUARANTINE_DATA
} FailureStrategy;

FailureStrategy handle_partition_failure(PartitionError* error) {
    if (error->type == ERROR_MEMORY) {
        // Try with smaller batch size
        return RETRY_WITH_BACKOFF;
    } else if (error->type == ERROR_CORRUPT_DATA) {
        // Move to quarantine for investigation
        return QUARANTINE_DATA;
    } else if (error->type == ERROR_PARSING && !get_env_fail_fast()) {
        // Skip this partition if not critical
        return SKIP_PARTITION;
    }
    return FAIL_FAST;
}
```

## PERFORMANCE TARGETS

### Processing Benchmarks
- **Small surveys (<100MB)**: < 5 seconds
- **Medium surveys (100MB-1GB)**: < 30 seconds  
- **Large surveys (1GB-5GB)**: < 3 minutes
- **Memory usage**: < 2x input size for staging
- **CPU utilization**: > 80% on available cores

### Optimization Metrics
```c
typedef struct {
    double throughput_mbps;
    double records_per_second;
    double cpu_efficiency;
    double memory_efficiency;
    double io_wait_percentage;
    size_t cache_hit_rate;
} PerformanceMetrics;
```

## TESTING REQUIREMENTS

### 1. UNIT TESTS
- Partition strategy selection
- Memory arena allocation/deallocation
- Work-stealing algorithm
- Checkpointing/recovery

### 2. INTEGRATION TESTS
- End-to-end survey processing
- Parallel processing coordination
- Memory pressure scenarios
- Large file handling

### 3. STRESS TESTS
- 5GB+ file processing
- 60 concurrent surveys
- Memory exhaustion recovery
- Network/disk failure simulation

### 4. PERFORMANCE TESTS
- Throughput benchmarks
- Memory usage profiling
- CPU utilization analysis
- I/O bottleneck identification