# CLAUDE.md - BLS Rust Processing System

## PROJECT CONTEXT

Processing **60+ BLS surveys** totaling **45GB** of raw data, with individual surveys ranging from 28KB (cx) to 14GB (cs). The system must handle this extreme variance efficiently.

## ACTUAL DATA LANDSCAPE

### Size Distribution
```
Massive (>5GB):  cs(14GB), cb(7.8GB), ch(7.8GB)
Large (1-5GB):   nw(3.6GB), la(2.3GB), oe(1.8GB), sm(1.7GB), fw(1.5GB)
Medium (100MB-1GB): 20 surveys including ml(454MB), bd(422MB), ln(371MB)
Small (10-100MB): 25 surveys
Micro (<10MB):   15 surveys including cx(28KB), bg(80KB)
```

### File Structure Pattern
Every survey follows this structure:
```
{survey}/
├── {survey}.series         # Metadata (1KB to 3.6GB!)
├── data/                   # Data files
│   ├── {survey}.data.0.Current      # Latest data
│   ├── {survey}.data.1.AllData      # Historical
│   └── {survey}.data.{N}.{Category} # Split files
└── map/                    # Lookup tables
    ├── {survey}.contacts
    ├── {survey}.industry
    ├── {survey}.footnote
    └── {survey}.{lookup}
```

## CRITICAL OBSERVATIONS

### 1. Series File Sizes Vary Wildly
- **cb.series**: 3.6GB (!)
- **nw.series**: 1.2GB
- **oe.series**: 1.2GB
- Most others: < 10MB

This requires adaptive series processing with memory mapping for large series files.

### 2. Data File Splitting Patterns

**Geographic Splits** (sm survey example):
```
sm.data.1.Alabama
sm.data.5a.California
sm.data.5b.California
sm.data.5c.California  # California split into 3 files!
sm.data.33a.NewYork
sm.data.33b.NewYork  # New York split into 2 files
```

**Category Splits** (nd survey):
```
nd.data.4.Food
nd.data.10.Wood
nd.data.18.FabricatedMetalProduct
nd.data.22.TransportationEquipment
```

**Temporal Splits** (hs survey):
```
hs.data.1.1976to1980
hs.data.2.1981to1984
hs.data.3.1985to1988
```

### 3. Map File Complexity

Some surveys have simple lookups:
- **cx**: Only 4 map files
- **su**: Only 6 map files

Others have extensive lookups:
- **cb**: 22 map files including 123KB category file
- **ci**: Massive 11MB aspect file
- **wm**: 31MB aspect file (!!)

## ARCHITECTURE DECISIONS

### Memory Strategy by Survey Class

```rust
impl ProcessingStrategy {
    pub fn for_survey(code: &str, total_size: u64) -> Self {
        match code {
            // Massive surveys - exclusive processing
            "cs" | "cb" | "ch" => Self::Exclusive {
                memory_limit_gb: 16.0,
                use_mmap: true,
                dedicated_threads: num_cpus::get(),
            },
            
            // Large series files need special handling
            "nw" | "oe" if series_size > 1_000_000_000 => Self::MmapSeries {
                series_chunks: 16,
                data_strategy: Box::new(Self::streaming_strategy()),
            },
            
            // Surveys with huge aspect files
            "ci" | "cm" | "nb" | "wm" => Self::LookupOptimized {
                cache_lookups: true,
                lazy_load: true,
                compression: true,
            },
            
            // Standard processing for others
            _ => Self::standard_for_size(total_size),
        }
    }
}
```

### Parallel Execution Plan

```rust
pub struct ExecutionPlan {
    // Process sequentially (one at a time)
    pub exclusive: Vec<String>,     // ["cs", "cb", "ch"]
    
    // Process with limited parallelism (2 concurrent)
    pub large: Vec<String>,          // ["nw", "la", "oe", "sm", "fw"]
    
    // Process with moderate parallelism (4 concurrent)
    pub medium: Vec<String>,         // 20 surveys
    
    // Process with high parallelism (unlimited)
    pub small: Vec<String>,          // 40 surveys
}
```

## PROCESSING WORKFLOW

### Phase 1: Analysis & Planning
```bash
# Analyze all surveys and create execution plan
bls-processor analyze --data-dir data/raw/bls --output-plan plan.json

# Output shows:
# - Total data: 45GB
# - Surveys: 60
# - Estimated time: 15 minutes
# - Memory required: 16GB peak
# - Parallelization strategy: Adaptive
```

### Phase 2: Series Processing
```rust
// Special handling for large series files
for survey in surveys {
    let series_size = get_file_size(&format!("{}.series", survey));
    
    if series_size > 100_000_000 {  // >100MB
        process_series_mmap(survey).await?;
    } else {
        process_series_memory(survey).await?;
    }
}
```

### Phase 3: Data Processing
```rust
// Smart scheduling based on survey size
let scheduler = AdaptiveScheduler::new();

// Exclusive processing for massive surveys
for survey in ["cs", "cb", "ch"] {
    scheduler.process_exclusive(survey).await?;
    
    // Force garbage collection after each massive survey
    force_memory_cleanup();
}

// Parallel processing for others
scheduler.process_parallel(remaining_surveys).await?;
```

### Phase 4: Output Generation
```rust
// Adaptive output partitioning
let output_strategy = match survey_code {
    // Massive surveys: partition by year and state
    "cs" | "cb" | "ch" => OutputStrategy::MultiLevel {
        primary: Partition::ByYear,
        secondary: Partition::ByState,
        max_file_size_mb: 500,
    },
    
    // Geographic surveys: partition by state
    "sm" | "sa" | "la" => OutputStrategy::Geographic {
        level: GeoLevel::State,
    },
    
    // Small surveys: single file
    _ if total_size < 100_000_000 => OutputStrategy::SingleFile,
    
    // Default: partition by year
    _ => OutputStrategy::Temporal { 
        interval: TimeInterval::Year,
    },
};
```

## OPTIMIZATION TECHNIQUES

### 1. Smart File Reading
```rust
// For files with state splits (e.g., California split into 5 parts)
pub async fn read_state_files(state: &str, parts: Vec<PathBuf>) -> Result<DataFrame> {
    if parts.len() == 1 {
        read_single_file(&parts[0]).await
    } else {
        // Read parts in parallel and concatenate
        let futures: Vec<_> = parts
            .iter()
            .map(|path| read_file_async(path))
            .collect();
        
        let results = futures::future::join_all(futures).await;
        concatenate_dataframes(results?)
    }
}
```

### 2. Lookup Table Optimization
```rust
// Cache frequently used lookups
pub struct LookupCache {
    // Hot cache for frequent lookups (e.g., industry codes)
    hot: Arc<DashMap<String, String>>,
    
    // Memory-mapped large lookups (e.g., 31MB wm.aspect)
    mmap_tables: Arc<DashMap<String, Mmap>>,
    
    // Compressed storage for rarely used lookups
    cold: Arc<DashMap<String, Vec<u8>>>,
}
```

### 3. Memory Management
```rust
// Adaptive memory limits based on survey
pub fn get_memory_limit(survey: &str) -> MemoryLimit {
    match survey {
        "cs" | "cb" | "ch" => MemoryLimit::Fixed(16 * GB),
        "nw" | "oe" => MemoryLimit::Fixed(8 * GB),
        _ => MemoryLimit::Adaptive {
            min: 1 * GB,
            max: 4 * GB,
            target_usage: 0.7,
        },
    }
}
```

## COMMAND EXAMPLES

### Process Everything
```bash
# Full processing with optimal scheduling
bls-processor process-all \
    --data-dir data/raw/bls \
    --output-dir data/processed \
    --format parquet \
    --optimize-schedule \
    --checkpoint-interval 10m
```

### Process Specific Size Classes
```bash
# Process only massive surveys (one at a time)
bls-processor process \
    --surveys cs,cb,ch \
    --sequential \
    --memory-limit 16GB

# Process medium surveys in parallel
bls-processor process \
    --size-class medium \
    --parallel 4 \
    --memory-limit 4GB
```

### Debug Specific Survey
```bash
# Detailed processing of problematic survey
bls-processor debug \
    --survey cs \
    --verbose \
    --profile-memory \
    --checkpoint-every 1000000 \
    --output-metrics cs-metrics.json
```

## ERROR HANDLING

### Common Issues & Solutions

1. **Series file too large**
   - Auto-detected: cb.series (3.6GB)
   - Solution: Automatic mmap with chunked parsing

2. **State files split across multiple files**
   - Auto-detected: California in 5 parts
   - Solution: Parallel read and concatenation

3. **Huge lookup tables**
   - Auto-detected: wm.aspect (31MB)
   - Solution: Memory-mapped lazy loading

4. **Memory exhaustion on cs survey**
   - Solution: Exclusive processing mode
   - Forces sequential processing
   - Uses aggressive chunking

## PERFORMANCE EXPECTATIONS

Based on actual data:

| Survey | Size | Files | Expected Time | Memory Peak |
|--------|------|-------|--------------|-------------|
| cs | 14GB | 200+ | 4-5 min | 12GB |
| cb | 7.8GB | 150+ | 2-3 min | 8GB |
| ch | 7.8GB | 150+ | 2-3 min | 8GB |
| nw | 3.6GB | 2 | 90 sec | 4GB |
| sm | 1.7GB | 80+ | 60 sec | 2GB |
| All 60 | 45GB | 1463 | 15-20 min | 16GB |

## FILE COMBINING STRATEGY

### The Problem
Many surveys have 50-100+ small files that create I/O bottlenecks:
- **sm survey**: 80+ files (many state-specific)
- **sa survey**: 100+ files (state data)
- **cs survey**: 200+ files (various categories)

### The Solution: Intelligent File Combining

#### State File Combining (sm survey example)
```
Original structure:
sm.data.5a.California (7.9MB)
sm.data.5b.California (12MB)    → Combined: california_combined.parquet (48.9MB)
sm.data.5c.California (29MB)

sm.data.33a.NewYork (9.4MB)     → Combined: newyork_combined.parquet (23.4MB)
sm.data.33b.NewYork (14MB)
```

#### Small State Grouping
```
Small Northeast states:
sm.data.47.Vermont (3.6MB)
sm.data.30.NewHampshire (4.3MB)  → Combined: northeast_small.parquet (16.2MB)
sm.data.20.Maine (4.5MB)
sm.data.41.RhodeIsland (4.0MB)
```

### Combining Configuration (YAML)

```yaml
# In survey config file
combining:
  enabled: true
  strategies:
    # Auto-detect and combine split states
    - type: state_splits
      auto_detect: true
      combine_pattern: "([0-9]+[a-z]?)\.([A-Z][a-zA-Z]+)"
      
    # Group small files by region
    - type: geographic_grouping
      threshold_mb: 10
      group_by: census_region
      
    # Combine by category if small
    - type: category_grouping
      categories: [industry, occupation]
      max_combined_mb: 200

  # Combining rules
  rules:
    - preserve_boundaries: [year, data_type]  # Never combine across these
    - maintain_sort_order: true                # Keep data ordered
    - deduplicate_on: [series_id, year, period]
```

### Performance Impact

| Metric | Before Combining | After Combining | Improvement |
|--------|-----------------|-----------------|-------------|
| Files to process (sm) | 80 files | 45 groups | 44% fewer |
| I/O operations | 1,463 | ~500 | 66% reduction |
| Processing time (sm) | 60 seconds | 40 seconds | 33% faster |
| Memory efficiency | Fragmented | Pooled | 40% better |

## CRITICAL SUCCESS FACTORS

1. **Never load cs/cb/ch fully into memory** - Always use mmap + streaming
2. **Process exclusive surveys sequentially** - Prevents memory exhaustion
3. **Cache hot lookups** - Many surveys reuse industry/occupation codes
4. **Adaptive chunking** - Adjust based on available memory
5. **Checkpoint frequently** - 45GB takes time, enable resume on failure