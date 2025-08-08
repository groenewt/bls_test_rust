# Configuration Module

The Configuration module provides a comprehensive configuration management system for the Rusty BLS Data Processing application. It defines configuration models, loading mechanisms, and validation utilities for managing application settings and survey-specific configurations.

## Overview

The Configuration module is designed to:

1. Provide structured configuration models for different components and operations
2. Enable flexible configuration loading from multiple sources (YAML, JSON, environment variables)
3. Support configuration validation with detailed error reporting
4. Facilitate dynamic configuration updates and hot-reloading capabilities

## Module Structure

```
src/config/
├── mod.rs           # Module entry point and re-exports
├── model.rs         # Configuration data models and structures
├── loader.rs        # Configuration loading mechanisms
└── validator.rs     # Configuration validation logic
```

## Key Components

### Configuration Models (`model.rs`)

The `model.rs` file defines a hierarchy of configuration structures:

- `Config`: The main configuration struct containing all application settings
- Specific configuration structs for each component:
  - `SurveyConfig`: Survey-specific configuration settings
  - `ProcessingConfig`: Data processing configuration
  - `OutputConfig`: Output generation configuration
  - `ErrorHandlingConfig`: Error handling configuration
  - `PluginConfig`: Plugin system configuration

### Configuration Loader (`loader.rs`)

The `loader.rs` file provides functionality for loading configurations:

- `ConfigLoader`: Main configuration loading interface
- `YamlLoader`: YAML configuration file loader
- `JsonLoader`: JSON configuration file loader
- `EnvironmentLoader`: Environment variable configuration loader
- `CompositeLoader`: Combines multiple configuration sources

### Configuration Validator (`validator.rs`)

The `validator.rs` file provides validation capabilities:

- `ConfigValidator`: Main configuration validation interface
- `SchemaValidator`: Schema-based validation
- `BusinessRuleValidator`: Business logic validation
- `CrossReferenceValidator`: Cross-component validation

## Integration with Other Components

The Configuration module integrates with other components by:

1. Providing configuration models for each component
2. Offering centralized configuration management
3. Supporting dynamic configuration updates across the system

For more detailed information, see:

- [Tasks and Roadmap](tasks.md)
- [Integration Guidelines](integration.md)
- [Best Practices](best_practices.md)