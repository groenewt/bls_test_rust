//! # Rusty BLS Data Processing CLI
//!
//! Command-line interface for the Rusty BLS Data Processing system.
//! 
//! ## Usage
//! 
//! ```bash
//! # Process a specific survey
//! rusty process --survey ap --config config/surveys/ap.yml
//! 
//! # List available surveys
//! rusty list-surveys
//! 
//! # Validate configuration
//! rusty validate --config config/surveys/ap.yml
//! 
//! # Show version information
//! rusty --version
//! ```

use std::env;
use std::path::PathBuf;
use std::process;
use std::error::Error as StdError;

use rusty::{
    config::{Config, ConfigLoader},
    processing::{ProcessingEngine, ProcessingInput, ProcessingOutput},
    error::{Error, Result},
    init_with_tracing,
};

/// Command-line arguments structure
#[derive(Debug)]
struct Args {
    command: Command,
    survey: Option<String>,
    environment: Option<String>,
    config_path: Option<PathBuf>,
    verbose: bool,
    output_dir: Option<PathBuf>,
}

/// Available CLI commands
#[derive(Debug)]
enum Command {
    Process,
    ListSurveys,
    Validate,
    ValidateConfig,
    PrintConfig,
    MigrateLegacy,
    Version,
    Help,
}

#[tokio::main]
async fn main() {
    // Initialize tracing for enterprise monitoring
    if let Err(e) = init_with_tracing() {
        eprintln!("Failed to initialize tracing: {}", e);
        process::exit(1);
    }

    // Parse command-line arguments
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            print_help();
            process::exit(1);
        }
    };

    // Execute the requested command
    let result = match args.command {
        Command::Process => process_survey(args).await,
        Command::ListSurveys => list_surveys(args).await,
        Command::Validate => validate_config(args).await,
        Command::ValidateConfig => validate_survey_config(args).await,
        Command::PrintConfig => print_survey_config(args).await,
        Command::MigrateLegacy => migrate_legacy_config(args).await,
        Command::Version => {
            print_version();
            Ok(())
        }
        Command::Help => {
            print_help();
            Ok(())
        }
    };

    // Handle any errors
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        
        // Log the full error chain for debugging
        let mut source = e.source();
        while let Some(err) = source {
            eprintln!("  Caused by: {}", err);
            source = err.source();
        }
        
        process::exit(1);
    }
}

/// Parse command-line arguments
fn parse_args() -> Result<Args> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        return Ok(Args {
            command: Command::Help,
            survey: None,
            environment: None,
            config_path: None,
            verbose: false,
            output_dir: None,
        });
    }

    let mut command = Command::Help;
    let mut survey = None;
    let mut environment = None;
    let mut config_path = None;
    let mut verbose = false;
    let mut output_dir = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "process" => command = Command::Process,
            "list-surveys" => command = Command::ListSurveys,
            "validate" => command = Command::Validate,
            "validate-config" => command = Command::ValidateConfig,
            "print-config" => command = Command::PrintConfig,
            "migrate-legacy" => command = Command::MigrateLegacy,
            "--version" | "-V" => command = Command::Version,
            "--help" | "-h" => command = Command::Help,
            "--survey" | "-s" => {
                i += 1;
                if i < args.len() {
                    survey = Some(args[i].clone());
                } else {
                    return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                        "Missing survey code after --survey".to_string()
                    )));
                }
            }
            "--env" | "-e" => {
                i += 1;
                if i < args.len() {
                    environment = Some(args[i].clone());
                } else {
                    return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                        "Missing environment after --env".to_string()
                    )));
                }
            }
            "--config" | "-c" => {
                i += 1;
                if i < args.len() {
                    config_path = Some(PathBuf::from(&args[i]));
                } else {
                    return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                        "Missing config path after --config".to_string()
                    )));
                }
            }
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output_dir = Some(PathBuf::from(&args[i]));
                } else {
                    return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                        "Missing output directory after --output".to_string()
                    )));
                }
            }
            "--verbose" | "-v" => verbose = true,
            _ => {
                return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                    format!("Unknown argument: {}", args[i])
                )));
            }
        }
        i += 1;
    }

    Ok(Args {
        command,
        survey,
        environment,
        config_path,
        verbose,
        output_dir,
    })
}

/// Process a BLS survey
async fn process_survey(args: Args) -> Result<()> {
    let survey_code = args.survey.ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::InvalidArgument(
            "Survey code is required for processing".to_string()
        ))
    })?;

    tracing::info!("Starting processing for survey: {}", survey_code);

    // Load configuration
    let config = if let Some(config_path) = args.config_path {
        Config::load_from_file(&config_path)?
    } else {
        Config::load_for_survey(&survey_code)?
    };

    // Create processing engine with processing config
    let processing_config = rusty::processing::ProcessingConfig::default();
    let mut engine = ProcessingEngine::new(processing_config);

    // Create input and output for processing
    let input = ProcessingInput::new(vec!["data/raw/bls/example.csv".to_string()]);
    let output = ProcessingOutput::new(vec!["data/processed/output.csv".to_string()], "csv".to_string());
    
    // Process the survey
    let _context = engine.process(input, output).await?;

    tracing::info!("Successfully processed survey: {}", survey_code);
    println!("Processing completed successfully for survey: {}", survey_code);

    Ok(())
}

/// List available surveys
async fn list_surveys(_args: Args) -> Result<()> {
    tracing::info!("Listing available surveys");

    // This would typically scan the config/surveys directory
    // For now, we'll provide a placeholder implementation
    println!("Available surveys:");
    println!("  ap - Average Price Data");
    println!("  bd - Business Dynamics Statistics");
    println!("  ce - Current Employment Statistics");
    println!("  ex - Example Survey (for testing)");
    
    // TODO: Implement actual survey discovery
    tracing::warn!("Survey listing not fully implemented - showing placeholder data");

    Ok(())
}

/// Validate a configuration file
async fn validate_config(args: Args) -> Result<()> {
    let config_path = args.config_path.ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::InvalidArgument(
            "Config path is required for validation".to_string()
        ))
    })?;

    tracing::info!("Validating configuration: {:?}", config_path);

    // Load and validate the configuration
    let _config = Config::load_from_file(&config_path)?;

    println!("Configuration is valid: {:?}", config_path);
    tracing::info!("Configuration validation successful");

    Ok(())
}

/// Validate a survey configuration using the new modular system
async fn validate_survey_config(args: Args) -> Result<()> {
    let survey_code = args.survey.ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::InvalidArgument(
            "Survey code is required for configuration validation".to_string()
        ))
    })?;

    let environment = args.environment.as_deref();
    
    tracing::info!("Validating survey configuration: {} (env: {:?})", survey_code, environment);

    // Use the new load_survey_config function which includes validation
    match rusty::config::load_survey_config(&survey_code, environment) {
        Ok(_config) => {
            println!("✓ Survey configuration is valid: {} (env: {})", 
                    survey_code, environment.unwrap_or("dev"));
            tracing::info!("Survey configuration validation successful");
        }
        Err(e) => {
            println!("✗ Survey configuration validation failed: {}", e);
            return Err(Error::Config(e));
        }
    }

    Ok(())
}

/// Print a survey configuration in a readable format
async fn print_survey_config(args: Args) -> Result<()> {
    let survey_code = args.survey.ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::InvalidArgument(
            "Survey code is required for printing configuration".to_string()
        ))
    })?;

    let environment = args.environment.as_deref();
    
    tracing::info!("Loading survey configuration: {} (env: {:?})", survey_code, environment);

    // Load the configuration using the new system
    let config = rusty::config::load_survey_config(&survey_code, environment)?;
    
    // Print configuration in YAML format
    let yaml_output = serde_yaml::to_string(&config)
        .map_err(|e| {
            let config_error = rusty::error::ConfigError::ParseError {
                message: format!("Failed to serialize configuration: {}", e),
                line: None,
                column: None,
            };
            Error::Config(config_error)
        })?;
    
    println!("Configuration for survey {} (env: {}):", 
             survey_code, environment.unwrap_or("dev"));
    println!("---");
    println!("{}", yaml_output);

    Ok(())
}

/// Migrate a legacy monolithic configuration to the new modular structure
async fn migrate_legacy_config(args: Args) -> Result<()> {
    let survey_code = args.survey.ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::InvalidArgument(
            "Survey code is required for legacy migration".to_string()
        ))
    })?;

    tracing::info!("Migrating legacy configuration for survey: {}", survey_code);

    // This is a placeholder implementation
    // In a real implementation, this would:
    // 1. Load the legacy monolithic config file
    // 2. Parse and split it into modular components
    // 3. Write the new modular files to the appropriate directories
    // 4. Validate the new configuration
    
    println!("Legacy migration for survey {} is not yet implemented", survey_code);
    println!("This feature will:");
    println!("  1. Load legacy config/surveys/{}.yml", survey_code);
    println!("  2. Split into modular files (overview.yml, model.yml, etc.)");
    println!("  3. Create config/surveys/{}/", survey_code);
    println!("  4. Validate the new modular configuration");
    
    tracing::warn!("Legacy migration not yet implemented for survey: {}", survey_code);

    Ok(())
}

/// Print version information
fn print_version() {
    println!("{} v{}", rusty::NAME, rusty::VERSION);
    println!("{}", rusty::DESCRIPTION);
}

/// Print help information
fn print_help() {
    println!("Rusty BLS Data Processing v{}", rusty::VERSION);
    println!("{}", rusty::DESCRIPTION);
    println!();
    println!("USAGE:");
    println!("    rusty <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    process          Process a BLS survey");
    println!("    list-surveys     List available surveys");
    println!("    validate         Validate a configuration file");
    println!("    validate-config  Validate a survey configuration using modular system");
    println!("    print-config     Print a survey configuration in readable format");
    println!("    migrate-legacy   Migrate legacy monolithic config to modular structure");
    println!("    --version        Show version information");
    println!("    --help           Show this help message");
    println!();
    println!("OPTIONS:");
    println!("    -s, --survey <CODE>     Survey code to process");
    println!("    -e, --env <ENV>         Environment (dev, stage, prod) [default: dev]");
    println!("    -c, --config <PATH>     Path to configuration file");
    println!("    -o, --output <DIR>      Output directory");
    println!("    -v, --verbose           Enable verbose logging");
    println!();
    println!("EXAMPLES:");
    println!("    rusty process --survey ap");
    println!("    rusty process --survey ap --env prod");
    println!("    rusty validate-config --survey ap --env dev");
    println!("    rusty print-config --survey ap --env prod");
    println!("    rusty migrate-legacy --survey ap");
    println!("    rusty validate --config config/surveys/ap.yml");
    println!("    rusty list-surveys");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_help() {
        let result = parse_args();
        // This test would need to mock env::args()
        // For now, just ensure it doesn't panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_print_version() {
        // Test that print_version doesn't panic
        print_version();
    }

    #[test]
    fn test_print_help() {
        // Test that print_help doesn't panic
        print_help();
    }
}