# Error Handling Module: Best Practices

This document outlines best practices for using the Error Handling module in the Rusty BLS Data Processing system.

## Error Type Design

### 1. Use Specific Error Types

Create specific error types for different error categories:

✅ **Good**:
```rust
enum DataError {
    parse_error { message: String, line: Option<usize> },
    ValidationError { message: String },
    MissingFile { path: PathBuf },
}
```

❌ **Bad**:
```rust
enum DataError {
    Error(String), // Too generic
}
```

### 2. Include Relevant Context

Include relevant context in error variants:

✅ **Good**:
```rust
enum ConfigError {
    MissingValue {
        key: String,
        path: Option<PathBuf>,
    },
}
```

❌ **Bad**:
```rust
enum ConfigError {
    MissingValue, // Missing context
}
```

### 3. Use Structured Errors

Use structured errors rather than string messages:

✅ **Good**:
```rust
enum ProcessingError {
    StrategyError {
        strategy: String,
        message: String,
    },
}
```

❌ **Bad**:
```rust
enum ProcessingError {
    Error(String), // Just a string message
}
```

## Error Handling Patterns

### 1. Use the `?` Operator

Use the `?` operator for concise error propagation:

✅ **Good**:
```rust
fn process_file(path: &Path) -> Result<Data> {
    let file = File::open(path)?;
    let data = parse_file(file)?;
    Ok(data)
}
```

❌ **Bad**:
```rust
fn process_file(path: &Path) -> Result<Data> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Err(e.into()),
    };
    
    let data = match parse_file(file) {
        Ok(d) => d,
        Err(e) => return Err(e.into()),
    };
    
    Ok(data)
}
```

### 2. Add Context to Errors

Add context to errors to make them more informative:

✅ **Good**:
```rust
fn load_survey(code: &str) -> Result<Survey> {
    let path = format!("data/{}.yml", code);
    let file = File::open(&path)
        .with_context(format!("Failed to open survey file for '{}'", code))?;
    
    // Rest of the implementation
    Ok(Survey::new())
}
```

❌ **Bad**:
```rust
fn load_survey(code: &str) -> Result<Survey> {
    let path = format!("data/{}.yml", code);
    let file = File::open(&path)?; // No context
    
    // Rest of the implementation
    Ok(Survey::new())
}
```

### 3. Handle Errors at the Appropriate Level

Handle errors at the level where you have enough context:

✅ **Good**:
```rust
// Low-level function
fn parse_value(input: &str) -> Result<Value> {
    // Just propagate the error
    let value = input.parse::<f64>()?;
    Ok(Value::Number(value))
}

// Higher-level function
fn process_survey(code: &str) -> Result<()> {
    match load_survey(code) {
        Ok(survey) => {
            // Process the survey
            Ok(())
        }
        Err(err) => {
            // Log the error with context
            log::error!("Failed to process survey {}: {}", code, err);
            
            // Maybe try a fallback
            if let Some(fallback) = get_fallback_survey(code) {
                process_fallback_survey(fallback)
            } else {
                Err(err)
            }
        }
    }
}
```

❌ **Bad**:
```rust
// Low-level function handling errors inappropriately
fn parse_value(input: &str) -> Result<Value> {
    match input.parse::<f64>() {
        Ok(value) => Ok(Value::Number(value)),
        Err(_) => {
            // Handling error at too low a level
            log::error!("Failed to parse value: {}", input);
            Ok(Value::Number(0.0)) // Silent fallback
        }
    }
}
```

## Error Reporting

### 1. Use Structured Logging

Use structured logging for errors:

✅ **Good**:
```rust
// Example of structured logging
fn report_error(code: &str, err: &Error) {
    log::error!(
        target: "survey_processing",
        survey_code = code,
        error = %err,
        "Failed to process survey"
    );
}
```

❌ **Bad**:
```rust
// Example of unstructured logging
fn report_error(code: &str, err: &Error) {
    println!("Error: {}", err); // Unstructured, not captured by logging system
}
```

### 2. Include Relevant Information

Include all relevant information in error messages:

✅ **Good**:
```rust
// Example of a detailed error
fn create_parse_error(message: &str, line: usize, column: usize, path: &Path) -> DataError {
    DataError::parse_error {
        message: message.to_string(),
        line: Some(line),
        column: Some(column),
        path: Some(path.to_path_buf()),
    }
}
```

❌ **Bad**:
```rust
// Example of a vague error
fn create_parse_error(message: &str) -> DataError {
    DataError::parse_error {
        message: "Error parsing".to_string(), // Too vague
        line: None,
        column: None,
        path: None,
    }
}
```

### 3. Use Error Codes for External Reporting

Use error codes for external reporting:

✅ **Good**:
```rust
// Example of error codes in API responses
fn format_api_error(err: &Error) -> serde_json::Value {
    let (code, message) = match err {
        Error::Auth(_) => ("E1001", "Authentication error"),
        Error::Data(_) => ("E2001", "Data processing error"),
        // Other error types
        _ => ("E9999", "Internal server error"),
    };
    
    serde_json::json!({
        "error": {
            "code": code,
            "message": message
        }
    })
}
```

❌ **Bad**:
```rust
// Example of exposing raw error details
fn format_api_error(err: &Error) -> String {
    format!("Failed to process request: {:?}", err)
}
```

## Performance Optimization

### 1. Minimize Allocations

Minimize allocations in error paths:

✅ **Good**:
```rust
// Using static strings
const INVALID_FORMAT_MSG: &str = "Invalid format";

fn validate_format(input: &str) -> Result<()> {
    if !is_valid_format(input) {
        return Err(DataError::ValidationError {
            message: INVALID_FORMAT_MSG.to_string(),
            path: None,
        }.into());
    }
    Ok(())
}
```

❌ **Bad**:
```rust
fn validate_format(input: &str) -> Result<()> {
    if !is_valid_format(input) {
        // Creating a new string for every error
        return Err(DataError::ValidationError {
            message: format!("The input '{}' has an invalid format", input),
            path: None,
        }.into());
    }
    Ok(())
}
```

### 2. Avoid Deep Error Chains

Avoid creating deep error chains that could impact performance:

✅ **Good**:
```rust
// Flatten error chains when appropriate
fn process_data(input: &str) -> Result<Output> {
    match parse_input(input) {
        Ok(data) => transform_data(data),
        Err(err) => {
            // Create a new error with the essential information
            Err(DataError::parse_error {
                message: format!("Failed to parse input: {}", err),
                line: None,
                column: None,
                path: None,
            }.into())
        }
    }
}
```

❌ **Bad**:
```rust
// Creating deep error chains
fn process_data(input: &str) -> Result<Output> {
    let data = parse_input(input)
        .with_context("Failed to parse input")?;
    
    transform_data(data)
        .with_context("Failed to transform data")?
        .with_context("Failed to process data")
}
```

### 3. Use Error Pools for High-Frequency Errors

Consider using error pools for high-frequency errors:

✅ **Good**:
```rust
// Using an error pool (conceptual example)
struct ErrorPool {
    validation_errors: Vec<DataError>,
    // Other error types
}

impl ErrorPool {
    fn get_validation_error(&mut self, msg: &str) -> DataError {
        if let Some(mut error) = self.validation_errors.pop() {
            // Reuse an existing error
            match &mut error {
                DataError::ValidationError { message, .. } => {
                    *message = msg.to_string();
                }
                _ => {}
            }
            error
        } else {
            // Create a new error
            DataError::ValidationError {
                message: msg.to_string(),
                path: None,
            }
        }
    }
    
    fn recycle(&mut self, error: DataError) {
        match error {
            DataError::ValidationError { .. } => self.validation_errors.push(error),
            // Handle other error types
            _ => {}
        }
    }
}
```

## Security Considerations

### 1. Avoid Exposing Sensitive Information

Avoid exposing sensitive information in error messages:

✅ **Good**:
```rust
fn authenticate(username: &str, password: &str) -> Result<User> {
    if !is_valid_credentials(username, password) {
        return Err(AuthError::InvalidCredentials {
            username: username.to_string(),
            // Don't include the password in the error
        }.into());
    }
    // Rest of the implementation
    Ok(User::new())
}
```

❌ **Bad**:
```rust
fn authenticate(username: &str, password: &str) -> Result<User> {
    if !is_valid_credentials(username, password) {
        return Err(AuthError::InvalidCredentials {
            username: username.to_string(),
            password: password.to_string(), // Including sensitive information
        }.into());
    }
    // Rest of the implementation
    Ok(User::new())
}
```

### 2. Sanitize Error Messages

Sanitize error messages before exposing them to users:

✅ **Good**:
```rust
fn handle_api_error(err: Error) -> ApiResponse {
    let (code, message) = match err {
        Error::Auth(AuthError::InvalidCredentials { .. }) => {
            ("E1001", "Invalid username or password")
        }
        Error::Data(DataError::ValidationError { message, .. }) => {
            ("E2001", &sanitize_message(&message))
        }
        // Other error types
        _ => ("E9999", "An internal error occurred"),
    };
    
    ApiResponse::error(code, message)
}

fn sanitize_message(message: &str) -> String {
    // Remove any sensitive information
    // This is a simplified example
    message.replace(|c: char| c == '\'' || c == '"', "")
}
```

❌ **Bad**:
```rust
fn handle_api_error(err: Error) -> ApiResponse {
    // Directly exposing the error message
    ApiResponse::error("ERROR", &format!("{}", err))
}
```

### 3. Implement Error Rate Limiting

Implement error rate limiting to prevent DoS attacks:

✅ **Good**:
```rust
struct ErrorRateLimiter {
    counters: HashMap<String, (usize, Instant)>,
    threshold: usize,
    window: Duration,
}

impl ErrorRateLimiter {
    fn check(&mut self, error_type: &str) -> bool {
        let now = Instant::now();
        let entry = self.counters.entry(error_type.to_string())
            .or_insert((0, now));
        
        if now.duration_since(entry.1) > self.window {
            // Reset the counter if the window has passed
            *entry = (1, now);
            true
        } else if entry.0 < self.threshold {
            // Increment the counter
            entry.0 += 1;
            true
        } else {
            // Rate limit exceeded
            false
        }
    }
}

fn handle_request(req: Request) -> Response {
    static mut ERROR_LIMITER: ErrorRateLimiter = ErrorRateLimiter {
        counters: HashMap::new(),
        threshold: 100,
        window: Duration::from_secs(60),
    };
    
    match process_request(req) {
        Ok(result) => Response::ok(result),
        Err(err) => {
            let error_type = match err {
                Error::Auth(_) => "auth",
                Error::Data(_) => "data",
                // Other error types
                _ => "other",
            };
            
            // Check if we should report this error
            let should_report = unsafe { ERROR_LIMITER.check(error_type) };
            
            if should_report {
                // Log and report the error
                log::error!("Request error: {}", err);
                Response::error(err)
            } else {
                // Just return a generic error
                Response::error_generic()
            }
        }
    }
}
```

## Testing Error Handling

### 1. Test Error Cases

Write tests for error cases:

✅ **Good**:
```rust
#[test]
fn test_parse_value_invalid_input() {
    let result = parse_value("not a number");
    assert!(result.is_err());
    
    match result {
        Err(Error::Data(DataError::parse_error { message, .. })) => {
            assert!(message.contains("Failed to parse"));
        }
        _ => panic!("Expected parse_error, got: {:?}", result),
    }
}
```

### 2. Test Error Recovery

Test error recovery mechanisms:

✅ **Good**:
```rust
#[test]
fn test_process_with_recovery() {
    // Test that the function recovers from validation errors
    let result = process_with_recovery("");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Default value");
    
    // Test that other errors are propagated
    let result = process_with_recovery_io("nonexistent.txt");
    assert!(result.is_err());
}
```

### 3. Test Error Context

Test that errors include the expected context:

✅ **Good**:
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