# BLS Creative Processing Architecture

## Core Philosophy: "Follow the Series"
The series file is the **blueprint** - it tells us how to organize everything else.

## Enhanced YAML Configuration with Partition Strategies

```yaml
# Example: Enhanced BD Survey Config
main:
  code: bd
  name: Business Employment Dynamics Data
  
# NEW: Smart Partition Configuration
partitioning:
  strategy: hierarchical  # none | single | hierarchical | adaptive
  
  # Define partition hierarchy (order matters!)
  hierarchy:
    - level: 1
      dimension: temporal
      field: year
      grouping: 5  # Group 5 years per partition
      
    - level: 2
      dimension: geographic
      field: state_code
      grouping: region  # Group states by region
      regions:
        northeast: [CT, ME, MA, NH, RI, VT, NJ, NY, PA]
        midwest: [IL, IN, MI, OH, WI, IA, KS, MN, MO, NE, ND, SD]
        south: [DE, FL, GA, MD, NC, SC, VA, DC, WV, AL, KY, MS, TN, AR, LA, OK, TX]
        west: [AZ, CO, ID, MT, NV, NM, UT, WY, AK, CA, HI, OR, WA]
        
    - level: 3
      dimension: industry
      field: industry_code
      grouping: supersector  # Group by NAICS supersector
      mapping:
        manufacturing: [31, 32, 33]
        trade: [42, 44, 45]
        services: [51, 52, 53, 54, 55, 56, 61, 62, 71, 72, 81]
        
  # Output organization
  output:
    structure: nested  # flat | nested | hybrid
    naming_pattern: "{year_range}/{region}/{supersector}/data.parquet"
    manifest: true
    include_metadata: true
    compression: snappy
    
  # Partition optimization hints
  optimization:
    target_partition_size_mb: 128
    max_partition_size_mb: 512
    min_records_per_partition: 10000
    balance_partitions: true
    
# Series Processing Instructions
series_processing:
  # How to decompose series_id
  decomposition:
    pattern: "BD{seasonal}{msa_code}{state_code}{county_code}{industry_code}{unitanalysis_code}{dataelement_code}{sizeclass_code}{dataclass_code}{ratelevel_code}{periodicity_code}{ownership_code}"
    fields:
      - name: seasonal
        start: 2
        length: 1
      - name: state_code
        start: 7
        length: 2
        lookup: bd.state
      - name: industry_code
        start: 15
        length: 6
        lookup: bd.industry
      # ... etc
        
  # Index building strategy
  indexing:
    primary_key: series_id
    secondary_indexes:
      - field: state_code
        type: hash
      - field: industry_code
        type: btree
      - field: [state_code, industry_code]
        type: composite
        
  # Memory strategy for series
  memory_strategy: adaptive  # all | indexed | streaming
```

## Innovative Processing Pipeline

```c
// Creative partition strategy structure
typedef struct {
    enum {
        PARTITION_NONE,           // Single output file
        PARTITION_SINGLE,         // Single dimension partitioning
        PARTITION_HIERARCHICAL,   // Multi-level hierarchy
        PARTITION_ADAPTIVE        // Learn from data
    } strategy_type;
    
    // Hierarchical partition levels
    struct PartitionLevel {
        int level;
        char dimension[32];      // temporal, geographic, industry, etc.
        char field[64];          // Field to partition on
        
        enum {
            GROUP_INDIVIDUAL,     // Each value gets own partition
            GROUP_RANGE,         // Range grouping (e.g., years)
            GROUP_MAPPING,       // Custom mapping (e.g., regions)
            GROUP_DYNAMIC        // Determine from data distribution
        } grouping_type;
        
        union {
            int range_size;      // For GROUP_RANGE
            HashMap* mapping;    // For GROUP_MAPPING
            void* dynamic_params;// For GROUP_DYNAMIC
        } grouping_params;
        
    } levels[MAX_PARTITION_LEVELS];
    int level_count;
    
    // Output organization
    struct {
        char naming_pattern[256];
        bool create_manifest;
        bool include_metadata;
        CompressionType compression;
    } output_config;
    
} PartitionStrategy;
```

## Three-Stage Processing Flow

### Stage 1: Series Analysis & Index Building

```c
typedef struct {
    // Series decomposition based on YAML pattern
    SeriesComponent components[MAX_COMPONENTS];
    int component_count;
    
    // Multi-dimensional index
    struct {
        HashIndex* by_id;              // O(1) lookup by series_id
        TreeIndex* by_time;            // Temporal ordering
        SpatialIndex* by_geography;    // Geographic clustering
        InvertedIndex* by_industry;    // Industry grouping
    } indexes;
    
    // Partition hints discovered from series
    struct {
        int distinct_states;
        int distinct_industries;
        int year_range;
        size_t estimated_records_per_series;
        DataDistribution* distributions;
    } statistics;
    
} SeriesAnalysis;

// Analyze series to optimize partitioning
SeriesAnalysis* analyze_series(const char* series_file, const YamlConfig* config) {
    SeriesAnalysis* analysis = create_analysis();
    
    // Stream through series file
    SeriesStream* stream = open_series_stream(series_file);
    
    while (has_next_series(stream)) {
        SeriesEntry entry = parse_series_entry(stream);
        
        // Decompose based on YAML pattern
        decompose_series_id(&entry, config->series_processing.decomposition);
        
        // Build indexes
        index_series_entry(analysis->indexes, &entry);
        
        // Collect statistics for partition optimization
        update_statistics(&analysis->statistics, &entry);
    }
    
    // Analyze data distribution
    analysis->statistics.distributions = analyze_distributions(analysis);
    
    return analysis;
}
```

### Stage 2: Dynamic Partition Planning

```c
typedef struct {
    char partition_id[128];         // Unique partition identifier
    char path_pattern[PATH_MAX];    // Output path for this partition
    
    // Partition boundaries
    struct {
        char dimension[32];
        char min_value[64];
        char max_value[64];
    } boundaries[MAX_PARTITION_LEVELS];
    int boundary_count;
    
    // Series belonging to this partition
    SeriesSet* series_set;          // Efficient set of series_ids
    size_t estimated_size_mb;
    size_t estimated_records;
    
    // Processing hints
    ProcessingStrategy optimal_strategy;
    size_t optimal_chunk_size;
    
} DataPartition;

typedef struct {
    DataPartition* partitions;
    int partition_count;
    
    // Partition tree for efficient lookup
    PartitionTree* tree;
    
    // Partition statistics
    struct {
        size_t total_partitions;
        size_t avg_partition_size_mb;
        size_t max_partition_size_mb;
        double balance_factor;  // 0.0 (unbalanced) to 1.0 (perfectly balanced)
    } stats;
    
} PartitionPlan;

// Create intelligent partition plan
PartitionPlan* create_partition_plan(SeriesAnalysis* analysis, PartitionStrategy* strategy) {
    PartitionPlan* plan = allocate_partition_plan();
    
    if (strategy->strategy_type == PARTITION_ADAPTIVE) {
        // Learn optimal partitioning from data
        strategy = learn_optimal_strategy(analysis);
    }
    
    // Build partition tree based on hierarchy
    plan->tree = build_partition_tree(strategy->levels, strategy->level_count);
    
    // Assign series to partitions
    for (each series in analysis) {
        DataPartition* partition = find_or_create_partition(plan->tree, series);
        add_series_to_partition(partition, series);
        
        // Split partition if it gets too large
        if (partition->estimated_size_mb > strategy->optimization.max_partition_size_mb) {
            split_partition(plan, partition);
        }
    }
    
    // Balance partitions if requested
    if (strategy->optimization.balance_partitions) {
        balance_partition_sizes(plan);
    }
    
    return plan;
}
```

### Stage 3: Parallel Data Processing with Smart Output

```c
typedef struct {
    DataPartition* partition;
    FILE* input_stream;
    ParquetWriter* output_writer;
    
    // Streaming buffer
    CircularBuffer* buffer;
    
    // Lookup cache for this partition
    LookupCache* local_cache;
    
    // Progress tracking
    atomic_size_t records_processed;
    atomic_size_t bytes_written;
    
} PartitionProcessor;

// Process all partitions in parallel
void process_data_files(PartitionPlan* plan, const char* data_dir) {
    ThreadPool* pool = create_thread_pool(get_optimal_thread_count());
    
    // Create processor for each partition
    PartitionProcessor* processors[plan->partition_count];
    
    for (int i = 0; i < plan->partition_count; i++) {
        processors[i] = create_partition_processor(plan->partitions[i]);
        
        // Submit to thread pool
        submit_task(pool, process_partition_task, processors[i]);
    }
    
    // Stream data files and route to correct partition
    DataFileStream* stream = open_data_files(data_dir);
    
    while (has_next_record(stream)) {
        DataRecord record = parse_record(stream);
        
        // Find target partition using tree lookup
        DataPartition* target = lookup_partition(plan->tree, record.series_id);
        
        // Route to correct processor
        route_record_to_processor(processors[target->index], record);
    }
    
    // Wait for completion
    wait_all_tasks(pool);
    
    // Generate manifest
    generate_output_manifest(plan);
}
```

## Output Organization Structure

```
output/
└── bd/                                 # Survey code
    ├── manifest.json                   # Complete manifest
    ├── metadata/
    │   ├── series_index.parquet       # Full series index
    │   ├── partition_map.json         # Partition structure
    │   ├── lookups/                   # Resolved lookup tables
    │   │   ├── states.parquet
    │   │   ├── industries.parquet
    │   │   └── sizeclasses.parquet
    │   └── statistics.json            # Data statistics
    │
    └── data/
        ├── 2020-2024/                  # Level 1: Temporal
        │   ├── northeast/              # Level 2: Geographic
        │   │   ├── manufacturing/      # Level 3: Industry
        │   │   │   ├── data.parquet
        │   │   │   └── _metadata.json
        │   │   ├── trade/
        │   │   │   ├── data.parquet
        │   │   │   └── _metadata.json
        │   │   └── services/
        │   │       ├── data.parquet
        │   │       └── _metadata.json
        │   ├── midwest/
        │   ├── south/
        │   └── west/
        ├── 2015-2019/
        └── 2010-2014/
```

## Manifest Format

```json
{
  "survey": "bd",
  "processing_date": "2024-01-15T10:30:00Z",
  "statistics": {
    "total_records": 15234567,
    "total_series": 12543,
    "date_range": "1992-2024",
    "partition_count": 240
  },
  "partition_strategy": {
    "type": "hierarchical",
    "levels": [
      {"dimension": "temporal", "field": "year", "grouping": 5},
      {"dimension": "geographic", "field": "state_code", "grouping": "region"},
      {"dimension": "industry", "field": "industry_code", "grouping": "supersector"}
    ]
  },
  "partitions": [
    {
      "id": "2020-2024/northeast/manufacturing",
      "path": "data/2020-2024/northeast/manufacturing/data.parquet",
      "size_mb": 127.3,
      "records": 234567,
      "series_count": 423,
      "boundaries": {
        "year": [2020, 2024],
        "states": ["CT", "ME", "MA", "NH", "RI", "VT", "NJ", "NY", "PA"],
        "industries": ["31", "32", "33"]
      }
    }
  ],
  "lookups": {
    "resolved": true,
    "tables": ["states", "industries", "sizeclasses", "dataclasses"]
  }
}
```

## Key Innovations

1. **Series-Driven Architecture**: Series file analysis determines optimal partitioning
2. **Hierarchical Partitioning**: Multi-level partitioning defined in YAML
3. **Adaptive Strategy**: System learns from data distribution
4. **Smart Routing**: Records automatically routed to correct partition
5. **Parallel Processing**: Each partition processed independently
6. **Rich Metadata**: Complete manifest and statistics for downstream use
7. **Flexible Output**: Nested, flat, or hybrid directory structures
8. **Lookup Resolution**: Pre-resolved lookup tables included in output

This architecture makes the partition strategy completely configurable while maintaining high performance and clear organization!