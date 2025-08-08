# Survey Configuration Migration Gameplan

## Overview
This document outlines the migration plan for converting existing monolithic survey configurations to the new modular, layered configuration structure as defined in the Survey Configuration Guide.

## Current State Analysis

### Monolithic Configuration Files (33 files)
These are individual .yml files in config/surveys/ that need to be migrated to modular structure:

1. bd.yml - Bureau of Labor Statistics survey
2. bg.yml - Bureau of Labor Statistics survey
3. bp.yml - Bureau of Labor Statistics survey
4. cb.yml - Bureau of Labor Statistics survey
5. cc.yml - Bureau of Labor Statistics survey
6. ce.yml - Bureau of Labor Statistics survey
7. cf.yml - Bureau of Labor Statistics survey
8. ch.yml - Bureau of Labor Statistics survey
9. ci.yml - Bureau of Labor Statistics survey
10. cm.yml - Bureau of Labor Statistics survey
11. cs.yml - Bureau of Labor Statistics survey
12. cu.yml - Bureau of Labor Statistics survey
13. cw.yml - Bureau of Labor Statistics survey
14. cx.yml - Bureau of Labor Statistics survey
15. eb.yml - Bureau of Labor Statistics survey
16. ex.yml - Example Survey (examined - 602 lines, complex structure)
17. fi.yml - Bureau of Labor Statistics survey
18. fm.yml - Bureau of Labor Statistics survey
19. fw.yml - Bureau of Labor Statistics survey
20. gg.yml - Bureau of Labor Statistics survey
21. gp.yml - Bureau of Labor Statistics survey
22. hc.yml - Bureau of Labor Statistics survey
23. hs.yml - Bureau of Labor Statistics survey
24. ii.yml - Bureau of Labor Statistics survey
25. in.yml - Bureau of Labor Statistics survey
26. ip.yml - Bureau of Labor Statistics survey
27. sm.yml - Bureau of Labor Statistics survey
28. tu.yml - Bureau of Labor Statistics survey
29. wm.yml - Bureau of Labor Statistics survey
30. wp.yml - Bureau of Labor Statistics survey
31. ws.yml - Bureau of Labor Statistics survey

### TEMPLATE Directory (needs cleanup)
The TEMPLATE directory contains partial modular configurations that need to be properly implemented:
- bls_template_configuration.yml
- data_partition_configuration.yml
- data_processing_configuration.yml
- data_processing_pipeline.yml
- quality_configuration.yml
- runtime_configuration.yml
- survey_template_configuration.yml
- survey_template_config.yml
- temp.txt

## Target Structure
Each survey will be migrated to the following modular structure:

```
config/surveys/<SURVEY_CODE>/
├── overview.yml            # Survey metadata and context
├── model.yml               # Data model: files, schemas, relationships
├── io.yml                  # Input discovery, patterns, partition hints
├── processing.yml          # Strategy, resources, chunking, mmap, cache
├── output.yml              # Output formats, partitioning, compression
├── dags.yml                # DAG tasks, dependencies, retries, SLAs
├── quality.yml             # Validations, expectations, thresholds
├── runtime.yml             # Feature flags, env overrides mapping, logging
└── overrides/
    ├── defaults.yml        # Org/team defaults for this survey
    ├── local.yml           # Developer machine overrides (gitignored)
    └── env/
        ├── dev.yml
        ├── stage.yml
        └── prod.yml
```

## Shared Infrastructure
Create shared base configurations:

```
config/surveys/_shared/
├── base_model.yml
├── base_processing.yml
├── base_output.yml
├── base_quality.yml
└── macros/
    ├── partitions.yml
    ├── parquet.yml
    └── mmap_profiles.yml
```

## Migration Strategy

### Phase 1: Infrastructure Setup
1. ✅ Create config/archive/surveys/ directory
2. Create config/surveys/_shared/ directory structure
3. Create shared base configurations and macros
4. Clean up TEMPLATE directory

### Phase 2: Pilot Migration (1 survey)
Start with **ex.yml** (Example Survey) as it's well-documented and complex:
1. Move ex.yml to config/archive/surveys/ex.yml
2. Create config/surveys/EX/ directory
3. Split ex.yml content into modular files
4. Test and validate the new structure

### Phase 3: Systematic Migration
Migrate remaining surveys in alphabetical order:
1. bd.yml → BD/
2. bg.yml → BG/
3. bp.yml → BP/
... (continue for all 33 surveys)

### Migration Process per Survey
For each survey:
1. **Backup**: Move original .yml to config/archive/surveys/
2. **Create Directory**: mkdir config/surveys/<SURVEY_CODE>/
3. **Split Configuration**: Extract content into modular files:
   - Extract survey metadata → overview.yml
   - Extract data models → model.yml
   - Extract input/discovery settings → io.yml
   - Extract processing strategy → processing.yml
   - Extract output settings → output.yml
   - Create basic DAG → dags.yml
   - Extract validation rules → quality.yml
   - Extract runtime settings → runtime.yml
4. **Create Overrides**: Set up overrides/ directory structure
5. **Validate**: Ensure new structure follows the guide
6. **Test**: Verify configuration can be loaded and processed

## Priority Order
1. **ex.yml** (pilot - most complex, well-documented)
2. **bd.yml** (if it's a commonly used survey)
3. **ce.yml** (Consumer Expenditure - mentioned in guide examples)
4. Remaining surveys alphabetically

## Validation Criteria
Each migrated survey must:
- Follow the exact directory structure defined in the guide
- Include config_version: 1 in each file
- Have proper YAML syntax
- Reference valid file paths and schemas
- Maintain all original functionality
- Include proper documentation

## Rollback Plan
- Original configurations backed up in config/archive/surveys/
- Can restore individual surveys if migration fails
- Git history provides additional safety net

## Success Metrics
- All 33 monolithic configs successfully migrated
- New modular structure validated
- No loss of functionality
- Improved maintainability and readability
- Shared configurations reduce duplication

## Next Steps
1. Create shared infrastructure
2. Begin pilot migration with ex.yml
3. Document lessons learned
4. Apply learnings to remaining surveys
5. Create validation tooling