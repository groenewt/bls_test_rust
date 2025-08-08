Here’s a concise plan followed by an updated guide that adopts the “temp.txt” direction.

High-level plan
- Adopt modular configuration: split monolithic survey config into overview.yml, model.yml, io.yml, processing.yml, output.yml, dags.yml, quality.yml, runtime.yml, plus overrides layers.
- Implement deep-merge with layered precedence: shared base → survey → overrides/defaults → overrides/env/<env> → overrides/local.
- Add per-file schema validation and cross-file validation (keys, relationships, partition/format compatibility, DAG tasks).
- Provide commands for list, validate, explain and CI integration with JSON reporting and golden “effective config” snapshots.
- Introduce shared macros and base profiles under config/surveys/_shared for reuse (e.g., parquet, partitions, mmap profiles).
- Enable versioned schemas per file, migrations, and evolution modes for outputs.
- Provide bootstrap templates and incremental adoption path with migration guidance.

```markdown
# Survey Configuration Guide (Modular, Layered, Validated)

This guide defines a modular, scalable approach for configuring surveys. It separates concerns into composable YAML files, supports environment/variant overlays, and enforces strong validation and linting for reliable operations at scale.

Goals
- Separation of concerns: split overview, model, IO, processing, output, DAG, quality, and runtime into distinct files.
- Composability: share reusable base configs across surveys and overlay survey- or environment-specific deltas.
- Versioning and evolution: enable per-file schema versions and controlled output schema evolution.
- Environment-aware: support dev/stage/prod overlays and runtime env var mapping.
- Strong validation: per-file schemas plus cross-file constraints and linters.

Recommended directory layout
- `config/surveys/<SURVEY_CODE>/`
  - `overview.yml`            — Survey metadata and context
  - `model.yml`               — Data model: files, schemas, relationships
  - `io.yml`                  — Input discovery, patterns, partition hints
  - `processing.yml`          — Strategy, resources, chunking, mmap, cache
  - `output.yml`              — Output formats, partitioning, compression
  - `dags.yml`                — DAG tasks, dependencies, retries, SLAs
  - `quality.yml`             — Validations, expectations, thresholds
  - `runtime.yml`             — Feature flags, env overrides mapping, logging
  - `overrides/`
    - `defaults.yml`          — Org/team defaults for this survey
    - `local.yml`             — Developer machine overrides (gitignored)
    - `env/`
      - `dev.yml`
      - `stage.yml`
      - `prod.yml`
  - `README.md`               — Optional notes for this survey

- `config/surveys/_shared/`   — Shared building blocks
  - `base_model.yml`
  - `base_processing.yml`
  - `base_output.yml`
  - `base_quality.yml`
  - `macros/`
    - `partitions.yml`
    - `parquet.yml`
    - `mmap_profiles.yml`

General conventions
- Every file includes a config version:
  - `config_version: 1` (increment to signal schema changes and migrations)
- Filenames lowercase with hyphens or underscores, e.g., `processing.yml`.
- The survey code is uppercase in content; the folder name matches code (`CS → config/surveys/CS`).
- References to shared anchors are allowed from `_shared/macros` (e.g., parquet defaults).
- Merge order (earlier is base, later overrides and wins):
  1) shared base → 2) survey files → 3) overrides/defaults → 4) overrides/env/<env> → 5) overrides/local
- Validation modes:
  - `strict`: fail on first error
  - `lenient`: collect all errors/warnings for a comprehensive report

File-by-file guidelines and templates

1) overview.yml
Purpose: human-centric metadata for readers and tools.
- yaml
```
yaml
config_version: 1
survey:
code: "CS"
name: "Consumer Expenditure Survey"
description: "Measures expenditures, income, and demographic characteristics."
size_class: "massive"      # micro|small|medium|large|massive
characteristics:
frequency: "monthly"
coverage: "US, state"
classification_system: "NAICS"
begin_year: 1984
update_schedule: "monthly"
contacts:
- role: "owner"
  name: "Team Name"
  email: "owner@example.org"
  documentation:
  primary: "docs/bls/survey/CS.txt"
  data_dictionary: "docs/bls/dictionaries/CS.md"
  tags: ["expenditure", "household", "time-series"]
```
2) model.yml
Purpose: data model: files, schemas, relationships, keys, and types.
- yaml
```
yaml
config_version: 1
series:
path: "cs.series"
index: "series_id"
estimated_size_gb: 2.0
schema:
series_id: { type: "string", length: 30, nullable: false }
seasonal:  { type: "string", length: 1, enum: ["S","U"], nullable: false }
# ...
data_files:
- id: "main"
  pattern: "cs.data.*"
  index: "series_id"
  schema:
  series_id: { type: "string", length: 30 }
  year:      { type: "int32",  min: 1900 }
  period:    { type: "string", pattern: "^(M\\d{2}|Q\\d{2}|A01)$" }
  value:     { type: "decimal", precision: 18, scale: 4 }
  footnote_codes: { type: "string", nullable: true }
  lookups:
- id: "industry"
  path: "cs.industry"
  index: "industry_code"
  schema:
  industry_code: { type: "string", length: 6 }
  industry_name: { type: "string", length: 100 }
  relationships:
- from: { file: "main", column: "industry_code" }
  to:   { file: "industry", column: "industry_code" }
  type: "many-to-one"
  constraints:
  unique:
    - { file: "series", columns: ["series_id"] }
      required_fields:
    - { file: "main", columns: ["series_id","year","period","value"] }
      types:
      aliases:
      Year:  { base: "int32",  range: [1900, 2100] }
      Money: { base: "decimal", precision: 18, scale: 2 }
```
3) io.yml
Purpose: input discovery, partition hints, and combining rules.
- yaml
```
yaml
config_version: 1
discovery:
root: "data/raw/bls/CS"
include: ["cs.series", "cs.data.*", "cs.industry"]
exclude: ["*.tmp", ".*", "*~"]
combining:
enabled: true
strategy: "smart"   # none|size_based|geographic|temporal|smart
thresholds:
small_file_mb: 10
partition_hints:
strategy: "adaptive"    # size_based|geographic|temporal|adaptive
max_partition_size_mb: 500
partition_by: ["year","state"]
mmap:
enable_for_mb_greater_than: 100
```
4) processing.yml
Purpose: strategy, concurrency, memory, cache, and transforms.
- yaml
```
yaml
config_version: 1
strategy:
mode: "exclusive"     # in_memory|chunked|parallel|streaming|exclusive
max_concurrency: 1
resources:
max_threads: 16
memory_limit_gb: 16
io:
chunk_size_mb: 64
buffer_pool_size: 8
features:
use_mmap: true
enable_statistics: true
enable_profiling: false
transforms:
- id: "normalize_period"
  expr: "upper(period)"
- id: "parse_value"
  expr: "to_decimal(value, 18, 4)"
```
5) output.yml
Purpose: output types, partitions, compression, and schema evolution rules.
- yaml
```
yaml
config_version: 1
format: "parquet"    # parquet|csv|json
parquet:
compression: "zstd"
compression_level: 3
row_group_size: 50000
enable_dictionary: true
bloom_filters: true
partitioning:
partition_by: ["year","state"]
max_file_size_mb: 512
naming:
dataset: "bls_cs"
version: "v1"
evolution:
mode: "compatible"   # strict|compatible|additive
allow_nullability_change: false
destination:
base_path: "data/processed/bls/CS"
```
6) dags.yml
Purpose: pipeline DAG with tasks, dependencies, retries, and SLAs.
- yaml
```
yaml
config_version: 1
graph:
tasks:
- id: "discover"
type: "scan"
inputs: []
retries: { max: 2, backoff_sec: 10 }
sla_minutes: 5
- id: "validate"
type: "validate_model"
inputs: ["discover"]
retries: { max: 1, backoff_sec: 5 }
- id: "plan"
type: "partition_plan"
inputs: ["validate"]
- id: "process"
type: "execute"
inputs: ["plan"]
parallelism: 2
- id: "write"
type: "output"
inputs: ["process"]
- id: "qc"
type: "quality_checks"
inputs: ["write"]
edges:
- from: "discover"
  to:   "validate"
- from: "validate"
  to:   "plan"
- from: "plan"
  to:   "process"
- from: "process"
  to:   "write"
- from: "write"
  to:   "qc"
```
7) quality.yml
Purpose: data quality expectations, validations, and thresholds.
- yaml
```
yaml
config_version: 1
expectations:
non_null:
- { file: "main", columns: ["series_id","year","period","value"] }
ranges:
- { file: "main", column: "year", min: 1900, max: 2100 }
enums:
- { file: "series", column: "seasonal", values: ["S","U"] }
regex:
- { file: "main", column: "period", pattern: "^(M\\d{2}|Q\\d{2}|A01)$" }
row_count:
min: 1
warn_below: 100
metrics:
collect: ["row_count","null_count","distinct(series_id)"]
persist: true
policy:
mode: "fail_on_error"   # warn_only|fail_on_error
sample_on_fail_rows: 50
```
8) runtime.yml
Purpose: runtime feature flags and environment mappings.
- yaml
```
yaml
config_version: 1
feature_flags:
enable_hot_reloads: false
use_memory_map_series: true
environment_overrides:
- key: "BLS_CS_MEMORY_LIMIT_GB"
  maps_to: "resources.memory_limit_gb"
- key: "BLS_OUTPUT_FORMAT"
  maps_to: "format"
- key: "BLS_PARTITION_STRATEGY"
  maps_to: "partition_hints.strategy"
  logging:
  level: "info"        # trace|debug|info|warn|error
  structured: true
```
Overrides and layering
- Shared base:
  - Place reusable defaults in `config/surveys/_shared/`.
  - Example profiles: `base_processing.yml` with small/medium/large presets.
- Survey-specific:
  - `config/surveys/<CODE>/` contains canonical configuration intent.
- Environment overlays:
  - `config/surveys/<CODE>/overrides/env/<env>.yml` holds environment deltas.
- Local overrides:
  - `config/surveys/<CODE>/overrides/local.yml` for developer machines (gitignored).
- Merge strategy:
  - Deep-merge with list de-duplication using a merge key (e.g., `id`).
  - On conflict, later layers win; log old/new values and sources.

Validation and linting
- Per-file schema validation:
  - Validate each YAML against its file schema at load time.
- Cross-file validation:
  - Model sanity:
    - `model.series.index` exists in `series.schema`.
    - `model.data_files[*].index` exists in the referenced schema.
    - Relationships reference valid file IDs and columns.
  - IO/Model compatibility:
    - `io.discovery.include` files must map to defined `model` entities.
  - Processing/Output consistency:
    - For parquet: `row_group_size > 0`.
    - `output.partitioning.partition_by` columns exist in model schemas.
  - DAG integrity:
    - Tasks reference supported types and valid dependencies; no cycles.
- Lints:
  - `survey.code` equals folder name (case-insensitive).
  - Forbid unknown top-level keys across files.
  - Enforce ID/file naming conventions.
  - Warn on unused lookups or relationships.

Scalability patterns for multiple “data configs”
- Variants per survey:
  - Use sibling variant folders (`<CODE>__variantA`) or overlays (`overrides/variant/<variant>.yml`).
  - Select via runtime arg (`--variant variantA`) or env variable.
- Multi-source surveys:
  - Multiple `data_files` with distinct IDs; `io.yml` may include multiple roots.
- Seasonal/data drift:
  - Change discovery patterns, partition strategies, or memory policies via environment overlays.
- Schema evolution/drift:
  - Versioned models: `model.v1.yml`, `model.v2.yml` with migration notes.
  - Maintain `migration.yml` describing field renames, type changes, defaults, and fill policies.

Developer commands
- `list`: summarize survey folders and validation status.
- `validate`: run per-file + cross-file checks; `--json` output for CI.
- `explain`: print effective config after applying overlays and env overrides.

CI workflow
- Validate all surveys/variants on each PR.
- Block merges on breaking changes unless accompanied by a migration plan.
- Maintain golden “effective config” snapshots per key surveys and diff on change.

Operational hints
- Discovery performance: cache large directory scans; invalidate on mtime changes.
- Structured logs: include fields — `survey_code`, `file`, `layer`, `key_path`, `action` (merged/overridden), `old_value`, `new_value`.
- Documentation: include succinct `README.md` in each survey folder with links and modeling notes.

Minimal bootstrap template

- yaml
```
yaml
# overview.yml
config_version: 1
survey:
code: "TEMPLATE"
name: "Template Survey"
description: "Short description"
size_class: "medium"
characteristics:
frequency: "monthly"
coverage: "US"
contacts: []
documentation: {}
tags: []
```
- yaml
```
yaml
# model.yml
config_version: 1
series:
path: ""
index: "series_id"
schema: {}
data_files: []
lookups: []
relationships: []
constraints: {}
types: {}
```
- yaml
```
yaml
# io.yml
config_version: 1
discovery:
root: "data/raw/bls/TEMPLATE"
include: []
exclude: ["*.tmp","*~",".*"]
combining:
enabled: false
partition_hints:
strategy: "size_based"
max_partition_size_mb: 512
partition_by: []
mmap:
enable_for_mb_greater_than: 100
```
- yaml
```
yaml
# processing.yml
config_version: 1
strategy: { mode: "chunked", max_concurrency: 2 }
resources: { max_threads: 4, memory_limit_gb: 4 }
io: { chunk_size_mb: 64, buffer_pool_size: 4 }
features: { use_mmap: true, enable_statistics: true, enable_profiling: false }
transforms: []
```
- yaml
```
yaml
# output.yml
config_version: 1
format: "parquet"
parquet:
compression: "snappy"
row_group_size: 100000
enable_dictionary: true
partitioning:
partition_by: []
max_file_size_mb: 512
naming: { dataset: "bls_template", version: "v1" }
evolution: { mode: "compatible", allow_nullability_change: false }
destination: { base_path: "data/processed/bls/TEMPLATE" }
```
- yaml
```
yaml
# dags.yml
config_version: 1
graph:
tasks:
- { id: "discover", type: "scan", inputs: [] }
- { id: "validate", type: "validate_model", inputs: ["discover"] }
- { id: "plan", type: "partition_plan", inputs: ["validate"] }
- { id: "process", type: "execute", inputs: ["plan"] }
- { id: "write", type: "output", inputs: ["process"] }
- { id: "qc", type: "quality_checks", inputs: ["write"] }
edges:
- { from: "discover", to: "validate" }
- { from: "validate", to: "plan" }
- { from: "plan", to: "process" }
- { from: "process", to: "write" }
- { from: "write", to: "qc" }
```
- yaml
```
yaml
# quality.yml
config_version: 1
expectations: {}
row_count: { min: 1 }
metrics: { collect: [], persist: false }
policy: { mode: "warn_only", sample_on_fail_rows: 20 }
```
- yaml
```
yaml
# runtime.yml
config_version: 1
feature_flags: { enable_hot_reloads: false, use_memory_map_series: false }
environment_overrides: []
logging: { level: "info", structured: true }
```
Incremental adoption
1) Split current monolithic config into `overview.yml`, `model.yml`, `processing.yml`, `output.yml`.
2) Move discovery/partition/combining into `io.yml`.
3) Introduce `quality.yml` for validations and checks.
4) Add `dags.yml` for orchestration.
5) Create `overrides/env/dev.yml` and `overrides/env/prod.yml`.
6) Implement a `validate` command that:
   - Validates each file against its schema.
   - Performs cross-file consistency checks.
   - Runs lint rules.
   - Outputs JSON for CI when requested.

Migration notes
- Preserve survey intent in the survey files; move environment differences to overlays.
- Use the `explain` command to verify the effective configuration after layering.
- Record schema changes via `config_version` increments and add `migration.yml` when breaking.

This structure positions you to scale to many surveys, variants, and evolving schemas while keeping each concern clean, testable, and maintainable.
```
