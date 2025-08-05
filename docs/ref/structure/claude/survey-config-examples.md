# Survey Configuration Examples

## SM Survey (State and Metro - 80+ files, 1.7GB total)

```yaml
# config/surveys/sm.yml
survey:
  code: sm
  name: State and Metro Area Employment
  description: Employment data by state and metropolitan area
  total_size_gb: 1.7
  file_count: 80

# Robust partition configuration
partition:
  # Grouping strategy for combining small files
  grouping:
    strategy: smart  # Automatically detect best grouping
    rules:
      # Combine state files that are split (e.g., California a,b,c)
      - type: state_splits
        action: combine
        max_combined_size_mb: 500
        output_pattern: "{state}_combined"
      
      # Group small state files by region
      - type: geographic
        threshold_mb: 10
        regions:
          northeast: [CT, ME, MA, NH, RI, VT, NJ, NY, PA]
          midwest: [IL, IN, MI, OH, WI, IA, KS, MN, MO, NE, ND, SD]
          south: [DE, FL, GA, MD, NC, SC, VA, DC, WV, AL, KY, MS, TN, AR, LA, OK, TX]
          west: [AZ, CO, ID, MT, NV, NM, UT, WY, AK, CA, HI, OR, WA]
        max_group_size_mb: 200
      
      # Category-based grouping
      - type: category
        patterns:
          total_nonfarm: "TotalNonFarm.*"
          goods_producing: "GoodsProducing.*"
          service_providing: "ServiceProviding.*"
          government: "Government.*"
        action: combine_if_small
        threshold_mb: 50
    
  # Processing strategy based on partition size
  processing:
    - size_range: [0, 10]  # MB
      strategy: in_memory
      combine: true
    - size_range: [10, 100]
      strategy: single_thread
      combine: true
    - size_range: [100, 500]
      strategy: parallel
      threads: 4
    - size_range: [500, null]
      strategy: streaming
      chunk_mb: 256

# File patterns and categorization
file_patterns:
  series: "sm.series"
  data:
    # Current aggregated data
    current: "sm.data.0.Current"
    
    # State-specific files with splits
    states:
      pattern: "sm.data.[0-9]+[a-z]?\\..*"
      examples:
        - "sm.data.1.Alabama"
        - "sm.data.5a.California"
        - "sm.data.5b.California"
        - "sm.data.5c.California"
        - "sm.data.33a.NewYork"
        - "sm.data.33b.NewYork"
    
    # Category files
    categories:
      pattern: "sm.data.[0-9]+\\.(TotalNonFarm|GoodsProducing|ServiceProviding|Government).*"
      examples:
        - "sm.data.54.TotalNonFarm.All"
        - "sm.data.57.GoodsProducing.Current"
        - "sm.data.76.Government.Current"
  
  # Lookup tables
  lookups:
    - sm.area
    - sm.state
    - sm.industry
    - sm.supersector
    - sm.data_type

# Data combining rules
combining:
  # Rules for merging data files
  rules:
    # California files (5a through 5c) should be combined
    - name: california_merge
      files: ["5a.California", "5b.California", "5c.California"]
      method: concatenate
      key_columns: [series_id, year, period]
      output: "california_combined"
    
    # New York files should be combined
    - name: newyork_merge  
      files: ["33a.NewYork", "33b.NewYork"]
      method: concatenate
      key_columns: [series_id, year, period]
      output: "newyork_combined"
    
    # Small northeastern states can be grouped
    - name: northeast_small
      size_threshold_mb: 5
      states: [VT, NH, ME, RI]
      method: union
      maintain_state_column: true
      output: "northeast_small_states"

# Output configuration
output:
  # Partition output by logical boundaries
  partitioning:
    primary: state        # Primary partition by state
    secondary: year       # Secondary partition by year
    
    # Override for combined files
    combined_files:
      partition_by: [region, year]
    
    # Maximum output file size
    max_file_size_mb: 500
    
  # Compression based on data type
  compression:
    default: snappy
    overrides:
      - pattern: "*_combined"
        compression: zstd
        level: 3
      - pattern: "historical_*"
        compression: zstd
        level: 6
  
  # Schema optimization
  schema:
    coalesce_small_files: true
    deduplicate_on: [series_id, year, period, value]
    sort_by: [state, year, period]
```

## CS Survey (Consumer Survey - 14GB, 200+ files)

```yaml
# config/surveys/cs.yml
survey:
  code: cs
  name: Consumer Survey
  description: Detailed consumer expenditure data
  total_size_gb: 14
  file_count: 200+

# Robust partition configuration for massive survey
partition:
  # Different strategy for massive survey
  grouping:
    strategy: hierarchical
    levels:
      # Level 1: Split by major categories
      - name: category_split
        type: pattern
        patterns:
          current: "cs.data.0.Current"
          historical: "cs.data.1.AllData"
          geographic: "cs.data.[0-9]+\\.[A-Z].*"
          industry: "cs.data.[0-9]+\\..*Industry.*"
          occupation: "cs.data.[0-9]+\\..*Occupation.*"
      
      # Level 2: Within each category, group by size
      - name: size_grouping
        type: adaptive
        rules:
          # Never combine files > 500MB
          - size_range_mb: [500, null]
            action: split
            target_chunk_mb: 256
          # Combine tiny files < 1MB
          - size_range_mb: [0, 1]
            action: combine
            max_combined_mb: 100
          # Process medium files individually
          - size_range_mb: [1, 500]
            action: individual
  
  # Special handling for this massive survey
  processing:
    mode: exclusive  # Process alone, no concurrent surveys
    memory_limit_gb: 16
    strategies:
      - size_range_mb: [0, 100]
        strategy: parallel_batch
        batch_size: 10
        threads_per_batch: 2
      - size_range_mb: [100, 1000]
        strategy: streaming
        chunk_mb: 128
        threads: 4
      - size_range_mb: [1000, null]
        strategy: mmap_chunked
        chunk_mb: 64
        parallel_chunks: 8

# Series file configuration (2GB!)
series:
  path: "cs.series"
  estimated_size_gb: 2.0
  processing:
    strategy: mmap_chunked
    chunk_size_mb: 64
    parallel_chunks: 16
    index_columns: [series_id, category, state]
  
# Memory management specific to CS
memory:
  # Aggressive memory management for massive survey
  mode: conservative
  limits:
    heap_mb: 8192
    mmap_mb: 8192
    buffer_pool_mb: 1024
  
  garbage_collection:
    frequency: every_100_files
    force_after_mb: 12288

# Combining rules for CS survey
combining:
  enabled: true
  strategies:
    # Combine small geographic files by region
    - type: geographic_aggregation
      regions:
        northeast: [CT, ME, MA, NH, RI, VT, NJ, NY, PA]
        southeast: [DE, FL, GA, MD, NC, SC, VA, DC, WV]
        midwest: [IL, IN, MI, OH, WI, IA, KS, MN, MO, NE, ND, SD]
        south: [AL, KY, MS, TN, AR, LA, OK, TX]
        west: [AZ, CO, ID, MT, NV, NM, UT, WY, AK, CA, HI, OR, WA]
      size_threshold_mb: 10
      output_pattern: "{region}_{category}"
    
    # Industry files can be combined if small
    - type: category_aggregation
      categories: [industry, occupation]
      max_combined_size_mb: 200
      maintain_category_column: true

# Output configuration for massive survey
output:
  # Multi-level partitioning for 14GB dataset
  partitioning:
    strategy: hierarchical
    levels:
      - column: year
        type: range
        ranges: [[2000, 2009], [2010, 2019], [2020, 2029]]
      - column: state
        type: hash
        buckets: 10
    
    # Ensure no single file exceeds 500MB
    max_file_size_mb: 500
    min_file_size_mb: 100  # Avoid too many tiny files
  
  compression:
    algorithm: zstd
    level: 3
    dictionary_size_kb: 32
  
  # Column-specific optimizations
  schema:
    encoding:
      series_id: dictionary
      state: dictionary
      year: delta
      value: delta_binary_packed
    
    statistics:
      enabled: true
      columns: [year, state, value]
    
    bloom_filters:
      enabled: true
      columns: [series_id, state]
      fpp: 0.01  # False positive probability
```

## JL Survey (Job Openings - 616KB, tiny survey)

```yaml
# config/surveys/jl.yml
survey:
  code: jl
  name: Job Openings and Labor Turnover
  description: Monthly job openings data
  total_size_gb: 0.0006
  file_count: 3

# Simple configuration for micro survey
partition:
  grouping:
    strategy: none  # Too small to partition
  
  processing:
    strategy: in_memory
    single_thread: true

# No combining needed - already minimal files
combining:
  enabled: false

# Simple output for tiny survey
output:
  partitioning:
    enabled: false  # Single output file
  
  compression:
    algorithm: snappy  # Fast compression for small data
  
  format: parquet
  single_file: true
  filename: "jl_complete.parquet"
```

## Smart Combining Configuration (General Template)

```yaml
# config/partition_strategies.yml
# Reusable partition strategies across surveys

strategies:
  # Strategy for surveys with many small state files
  state_heavy:
    grouping:
      primary: state
      combine_splits: true  # Auto-detect and combine split states
      group_small_states:
        threshold_mb: 10
        by_region: true
        regions:
          new_england: [CT, ME, MA, NH, RI, VT]
          mid_atlantic: [NJ, NY, PA]
          east_north_central: [IL, IN, MI, OH, WI]
          west_north_central: [IA, KS, MN, MO, NE, ND, SD]
          south_atlantic: [DE, FL, GA, MD, NC, SC, VA, DC, WV]
          east_south_central: [AL, KY, MS, TN]
          west_south_central: [AR, LA, OK, TX]
          mountain: [AZ, CO, ID, MT, NV, NM, UT, WY]
          pacific: [AK, CA, HI, OR, WA]
    
    output:
      partition_by: [region, year]
      maintain_state_column: true
  
  # Strategy for surveys with temporal splits
  time_series:
    grouping:
      primary: temporal
      combine_by:
        - decade: "19[0-9]{2}"
        - five_year: "20[0-2][0-9]"
      maintain_granularity: month
    
    output:
      partition_by: [decade, year]
      sort_by: [year, month]
  
  # Strategy for category-based surveys
  category_based:
    grouping:
      primary: category
      hierarchy:
        - level: sector
          examples: [goods, services, government]
        - level: industry
          examples: [manufacturing, retail, finance]
        - level: detailed
          examples: [auto_manufacturing, food_retail, investment_banking]
      
      combine_rules:
        - level: detailed
          if_smaller_than_mb: 5
          combine_to: industry
        - level: industry
          if_smaller_than_mb: 50
          combine_to: sector
    
    output:
      partition_by: [sector, industry]
      optimize_for: analytical_queries

# Global combining rules
global_rules:
  # Never combine across these boundaries
  preserve_boundaries:
    - year
    - survey_code
    - data_type  # Don't mix Current with Historical
  
  # Always combine these if found
  auto_combine:
    - pattern: "(.+)_part[0-9]+"  # Combine obvious parts
    - pattern: "(.+)[a-z]$"       # Combine letter-suffixed files
    
  # Size thresholds
  thresholds:
    min_file_size_mb: 1          # Files smaller than this are candidates for combining
    max_combined_size_mb: 500     # Never create combined files larger than this
    ideal_partition_size_mb: 256  # Target size for optimal processing
  
  # Performance hints
  hints:
    prefer_fewer_files: true      # Reduce I/O overhead
    maintain_sort_order: true     # Preserve data ordering
    deduplicate: true            # Remove duplicate records when combining
```