# Error Handling Module: Integration Guidelines

This document provides guidelines for integrating the Error Handling module with other components of the Rusty BLS Data Processing system.

## Basic Integration

### 1. Import the Error Types

To use the error handling module, import the necessary types:

```rust
use crate::error::{Error, Result};
use crate::error::{ConfigError, DataError, ProcessingError, OutputError, PluginError};
```

For context functionality:

```rust
use crate::error::{ErrorContext, ContextExt, OptionContextExt};
use crate::error_context;
```

### 2. Define Component-Specific Error Types

When creating a new component, define specific error types for that component:

```rust
// In your component's error.rs file
use std::fmt;
use std::error::Error as StdError;
use std::path::PathBuf;

#[derive(Debug)]
pub enum MyComponentError {
    // Define specific error variants for your component
    OperationFailed {
        operation: String,
        message: String,
    },
    ResourceNotFound {
        resource: String,
        path: Option<PathBuf>,
    },
    // Add more error variants as needed
    Other(String),
}

// Implement Display for your error type
impl fmt::Display for MyComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyComponentError::OperationFailed { operation, message } => {
                write!(f, "Operation '{}' failed: {}", operation, message)
            }
            MyComponentError::ResourceNotFound { resource, path } => {
                if let Some(path) = path {
                    write!(f, "Resource '{}' not found at {}", resource, path.display())
                } else {
                    write!(f, "Resource '{}' not found", resource)
                }
            }
            MyComponentError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

// Implement conversion to the main Error type
impl From<MyComponentError> for crate::error::Error {
    fn from(err: MyComponentError) -> Self {
        // Map to the appropriate error variant
        // This is just an example, adjust as needed
        crate::error::Error::Other(format!("MyComponent error: {}", err))
    }
}
```

### 3. Use Result Type for Error Handling

Use the `Result` type for functions that can fail:

```rust
use crate::error::Result;

pub fn process_data(input: &str) -> Result<String> {
    // Implementation that can return errors
    if input.is_empty() {
        return Err(DataError::ValidationError {
            message: "Input cannot be empty".to_string(),
            path: None,
        }.into());
    }
    
    // Process the data
    Ok(format!("Processed: {}", input))
}
```

### 4. Add Context to Errors

Use the context functionality to add more information to errors:

```rust
use crate::error::{ContextExt, error_context};

pub fn load_configuration(path: &str) -> Result<Config> {
    let file = std::fs::File::open(path)
        .with_context(format!("Failed to open configuration file: {}", path))?;
    
    let config = serde_yaml::from_reader(file)
        .with_context(format!("Failed to parse configuration file: {}", path))?;
    
    Ok(config)
}
```

Or using the macro:

```rust
pub fn load_configuration(path: &str) -> Result<Config> {
    let file = std::fs::File::open(path)
        .map_err(|e| error_context!("Failed to open configuration file", e))?;
    
    let config = serde_yaml::from_reader(file)
        .map_err(|e| error_context!("Failed to parse configuration file", e))?;
    
    Ok(config)
}
```

## Advanced Integration

### 1. Error Propagation

Use the `?` operator to propagate errors:

```rust
pub fn process_survey(survey_code: &str) -> Result<()> {
    let config = load_configuration(survey_code)?;
    let data = load_data(&config)?;
    process_data(&data)?;
    save_results(&data)?;
    Ok(())
}
```

### 2. Error Conversion

Implement `From` traits for converting between error types:

```rust
impl From<std::io::Error> for MyComponentError {
    fn from(err: std::io::Error) -> Self {
        MyComponentError::OperationFailed {
            operation: "I/O operation".to_string(),
            message: err.to_string(),
        }
    }
}
```

### 3. Custom Error Handling

Implement custom error handling when needed:

```rust
pub fn process_with_recovery(input: &str) -> Result<String> {
    match process_data(input) {
        Ok(result) => Ok(result),
        Err(err) => {
            // Log the error
            eprintln!("Error processing data: {}", err);
            
            // Try to recover
            if let Error::Data(DataError::ValidationError { .. }) = err {
                // For validation errors, use a default value
                Ok("Default value".to_string())
            } else {
                // For other errors, propagate
                Err(err)
            }
        }
    }
}
```

## Integration with External Libraries

### 1. Error Conversion from External Libraries

Implement `From` traits for converting errors from external libraries:

```rust
impl From<serde_json::Error> for MyComponentError {
    fn from(err: serde_json::Error) -> Self {
        MyComponentError::OperationFailed {
            operation: "JSON parsing".to_string(),
            message: err.to_string(),
        }
    }
}
```

### 2. Wrapping External Errors

Wrap external errors with context:

```rust
pub fn parse_json(input: &str) -> Result<Value> {
    serde_json::from_str(input)
        .with_context("Failed to parse JSON")?
        .into()
}
```

## Testing Error Handling

### 1. Testing Error Cases

Write tests for error cases:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_process_data_empty_input() {
        let result = process_data("");
        assert!(result.is_err());
        
        if let Err(Error::Data(DataError::ValidationError { message, .. })) = result {
            assert!(message.contains("Input cannot be empty"));
        } else {
            panic!("Expected ValidationError, got: {:?}", result);
        }
    }
}
```

### 2. Testing Error Context

Test that errors include the expected context:

```rust
#[test]
fn test_load_configuration_missing_file() {
    let result = load_configuration("nonexistent.yml");
    assert!(result.is_err());
    
    let err = result.unwrap_err();
    let err_string = format!("{}", err);
    assert!(err_string.contains("Failed to open configuration file"));
    assert!(err_string.contains("nonexistent.yml"));
}
```

## Performance Considerations

- Avoid excessive error context in hot paths
- Consider using static strings for error messages in performance-critical code
- Use error enums rather than string-based errors for better performance
- Minimize allocations in error handling paths

## Security Considerations

- Avoid including sensitive information in error messages
- Sanitize error messages before logging or displaying to users
- Consider using error codes for external error reporting
- Implement proper error isolation between components