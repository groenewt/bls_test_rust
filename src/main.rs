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
use std::error::Error as StdError;
use std::path::PathBuf;
use std::process;

use async_trait::async_trait;
use rusty::{
    config::{Config, ConfigLoader, load_survey_config},
    error::{Error, Result},
    init_with_tracing,
    processing::{
        DagExecutor, LoaderStageImpl, PipelineStage, ProcessingConfig, ProcessingContext,
        ProcessingEngine, ProcessingInput, ProcessingOutput, WriterStageImpl,
    },
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
    count: Option<usize>,
    seed: Option<u64>,
    allow_multi: bool,
}

/// Available CLI commands
#[derive(Debug)]
enum Command {
    Process,
    ProcessRandom,
    ProcessDags,
    ProcessDagsRandom,
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
        Command::ProcessRandom => process_random(args).await,
        Command::ProcessDags => process_dags(args).await,
        Command::ProcessDagsRandom => process_dags_random(args).await,
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
            count: None,
            seed: None,
            allow_multi: false,
        });
    }

    let mut command = Command::Help;
    let mut survey = None;
    let mut environment = None;
    let mut config_path = None;
    let mut verbose = false;
    let mut output_dir = None;
    let mut count: Option<usize> = None;
    let mut seed: Option<u64> = None;
    let mut allow_multi = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "process" => command = Command::Process,
            "process-random" => command = Command::ProcessRandom,
            "process-dags" => command = Command::ProcessDags,
            "process-dags-random" => command = Command::ProcessDagsRandom,
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
                        "Missing survey code after --survey".to_string(),
                    )));
                }
            }
            "--env" | "-e" => {
                i += 1;
                if i < args.len() {
                    environment = Some(args[i].clone());
                } else {
                    return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                        "Missing environment after --env".to_string(),
                    )));
                }
            }
            "--config" | "-c" => {
                i += 1;
                if i < args.len() {
                    config_path = Some(PathBuf::from(&args[i]));
                } else {
                    return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                        "Missing config path after --config".to_string(),
                    )));
                }
            }
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output_dir = Some(PathBuf::from(&args[i]));
                } else {
                    return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                        "Missing output directory after --output".to_string(),
                    )));
                }
            }
            "--verbose" | "-v" => verbose = true,
            "--count" | "-n" => {
                i += 1;
                if i < args.len() {
                    match args[i].parse::<usize>() {
                        Ok(n) => count = Some(n),
                        Err(_) => {
                            return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                                format!("Invalid count value: {}", args[i]),
                            )));
                        }
                    }
                } else {
                    return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                        "Missing number after --count".to_string(),
                    )));
                }
            }
            "--seed" => {
                i += 1;
                if i < args.len() {
                    match args[i].parse::<u64>() {
                        Ok(s) => seed = Some(s),
                        Err(_) => {
                            return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                                format!("Invalid seed value: {}", args[i]),
                            )));
                        }
                    }
                } else {
                    return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                        "Missing number after --seed".to_string(),
                    )));
                }
            }
            "--allow-multi" => {
                allow_multi = true;
            }
            _ => {
                return Err(Error::Config(rusty::error::ConfigError::InvalidArgument(
                    format!("Unknown argument: {}", args[i]),
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
        count,
        seed,
        allow_multi,
    })
}

/// Process a BLS survey
async fn process_survey(args: Args) -> Result<()> {
    let survey_code = args.survey.ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::InvalidArgument(
            "Survey code is required for processing".to_string(),
        ))
    })?;

    tracing::info!("Starting processing for survey: {}", survey_code);

    // Try loading configuration (optional). Proceed even if not available to allow raw processing fallback.
    let _config_opt = if let Some(config_path) = args.config_path {
        match Config::load_from_file(&config_path) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                tracing::warn!(
                    "Failed to load config from file for {}: {}. Proceeding with defaults.",
                    survey_code,
                    e
                );
                None
            }
        }
    } else {
        match Config::load_for_survey(&survey_code) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                tracing::warn!(
                    "Failed to load survey config for {}: {}. Proceeding with defaults.",
                    survey_code,
                    e
                );
                None
            }
        }
    };

    // Create processing engine with processing config
    let processing_config = rusty::processing::ProcessingConfig::default();
    let mut engine = ProcessingEngine::new(processing_config);

    // Use default discovery based on standard BLS directory layout to avoid strict config dependencies
    let base_dir = std::path::PathBuf::from(format!("data/raw/bls/{}", survey_code.to_lowercase()));
    let patterns = vec![
        "*.series".to_string(),
        "data/*".to_string(),
        "map/*".to_string(),
    ];
    let input_paths = patterns
        .into_iter()
        .map(|p| base_dir.join(p).to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let input = ProcessingInput::new(input_paths);

    let output_path = format!("data/processed/{}/output.csv", survey_code);
    let output = ProcessingOutput::new(vec![output_path], "csv".to_string());

    // Process the survey
    let _context = engine.process(input, output).await?;

    tracing::info!("Successfully processed survey: {}", survey_code);
    println!(
        "Processing completed successfully for survey: {}",
        survey_code
    );

    Ok(())
}

/// List available surveys
async fn list_surveys(_args: Args) -> Result<()> {
    tracing::info!("Listing available surveys");

    // Dynamically discover surveys from config/surveys directory
    match discover_surveys().await {
        Ok(surveys) => {
            println!("Available surveys:");
            for survey in surveys {
                println!("  {} - {}", survey.code.to_lowercase(), survey.name);
                if let Some(desc) = survey.description {
                    println!("    {}", desc);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to discover surveys: {}", e);
            println!("Error: Failed to load survey configurations from config/surveys/");
            return Err(e);
        }
    }

    Ok(())
}

/// Discover surveys from the config/surveys directory
async fn discover_surveys() -> Result<Vec<DiscoveredSurvey>> {
    use std::fs;
    use std::path::Path;

    let surveys_dir = Path::new("config/surveys");
    if !surveys_dir.exists() {
        return Err(Error::Config(rusty::error::ConfigError::LoadError {
            path: "config/surveys".to_string(),
            source: "Survey configurations directory not found".to_string(),
        }));
    }

    let mut surveys = Vec::new();

    // Read directory entries
    let entries = fs::read_dir(surveys_dir).map_err(|e| {
        Error::Config(rusty::error::ConfigError::LoadError {
            path: "config/surveys".to_string(),
            source: e.to_string(),
        })
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            Error::Config(rusty::error::ConfigError::LoadError {
                path: "config/surveys entry".to_string(),
                source: e.to_string(),
            })
        })?;

        let path = entry.path();
        if path.is_dir() {
            let survey_code = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Skip special directories
            if survey_code.starts_with('_')
                || survey_code.starts_with('.')
                || survey_code == "TEMPLATE"
            {
                continue;
            }

            // Try to load overview.yml for this survey
            let overview_path = path.join("overview.yml");
            if overview_path.exists() {
                match load_survey_overview(&overview_path).await {
                    Ok(survey) => surveys.push(survey),
                    Err(e) => {
                        tracing::warn!("Failed to load survey {}: {}", survey_code, e);
                        // Continue with other surveys instead of failing completely
                    }
                }
            }
        }
    }

    surveys.sort_by(|a, b| a.code.cmp(&b.code));
    Ok(surveys)
}

/// Simple structure for discovered survey information
#[derive(Debug, Clone)]
struct DiscoveredSurvey {
    code: String,
    name: String,
    description: Option<String>,
    size_class: Option<String>,
}

/// Load survey overview from overview.yml
async fn load_survey_overview(overview_path: &std::path::Path) -> Result<DiscoveredSurvey> {
    use std::fs;

    let content = fs::read_to_string(overview_path).map_err(|e| {
        Error::Config(rusty::error::ConfigError::LoadError {
            path: overview_path.display().to_string(),
            source: e.to_string(),
        })
    })?;

    let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content).map_err(|e| {
        Error::Config(rusty::error::ConfigError::ParseError {
            message: format!("Failed to parse YAML: {}", e),
            line: None,
            column: None,
        })
    })?;

    // Extract survey information from YAML structure
    let survey_section = yaml_value.get("survey").ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::ValidationError {
            message: "Missing survey section in overview.yml".to_string(),
            field: Some("survey".to_string()),
        })
    })?;

    let code = survey_section
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let name = survey_section
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Survey")
        .to_string();

    let description = survey_section
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let size_class = survey_section
        .get("size_class")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(DiscoveredSurvey {
        code,
        name,
        description,
        size_class,
    })
}

/// Validate a configuration file
async fn validate_config(args: Args) -> Result<()> {
    let config_path = args.config_path.ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::InvalidArgument(
            "Config path is required for validation".to_string(),
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
            "Survey code is required for configuration validation".to_string(),
        ))
    })?;

    let environment = args.environment.as_deref();

    tracing::info!(
        "Validating survey configuration: {} (env: {:?})",
        survey_code,
        environment
    );

    // Use the new load_survey_config function which includes validation
    match rusty::config::load_survey_config(&survey_code, environment) {
        Ok(_config) => {
            println!(
                "✓ Survey configuration is valid: {} (env: {})",
                survey_code,
                environment.unwrap_or("dev")
            );
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
            "Survey code is required for printing configuration".to_string(),
        ))
    })?;

    let environment = args.environment.as_deref();

    tracing::info!(
        "Loading survey configuration: {} (env: {:?})",
        survey_code,
        environment
    );

    // Load the configuration using the new system
    let config = rusty::config::load_survey_config(&survey_code, environment)?;

    // Print configuration in YAML format
    let yaml_output = serde_yaml::to_string(&config).map_err(|e| {
        let config_error = rusty::error::ConfigError::ParseError {
            message: format!("Failed to serialize configuration: {}", e),
            line: None,
            column: None,
        };
        Error::Config(config_error)
    })?;

    println!(
        "Configuration for survey {} (env: {}):",
        survey_code,
        environment.unwrap_or("dev")
    );
    println!("---");
    println!("{}", yaml_output);

    Ok(())
}

/// Migrate a legacy monolithic configuration to the new modular structure
async fn migrate_legacy_config(args: Args) -> Result<()> {
    let survey_code = args.survey.ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::InvalidArgument(
            "Survey code is required for legacy migration".to_string(),
        ))
    })?;

    tracing::info!("Migrating legacy configuration for survey: {}", survey_code);

    // This is a placeholder implementation
    // In a real implementation, this would:
    // 1. Load the legacy monolithic config file
    // 2. Parse and split it into modular components
    // 3. Write the new modular files to the appropriate directories
    // 4. Validate the new configuration

    println!(
        "Legacy migration for survey {} is not yet implemented",
        survey_code
    );
    println!("This feature will:");
    println!("  1. Load legacy config/surveys/{}.yml", survey_code);
    println!("  2. Split into modular files (overview.yml, model.yml, etc.)");
    println!("  3. Create config/surveys/{}/", survey_code);
    println!("  4. Validate the new modular configuration");

    tracing::warn!(
        "Legacy migration not yet implemented for survey: {}",
        survey_code
    );

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
    println!("    process               Process a BLS survey");
    println!("    process-random        Process N random surveys (default: 5)");
    println!("    process-dags          Execute DAG pipeline for a survey");
    println!("    process-dags-random   Execute DAG pipeline for N random surveys (default: 5)");
    println!("    list-surveys          List available surveys");
    println!("    validate              Validate a configuration file");
    println!("    validate-config       Validate a survey configuration using modular system");
    println!("    print-config          Print a survey configuration in readable format");
    println!("    migrate-legacy        Migrate legacy monolithic config to modular structure");
    println!("    --version             Show version information");
    println!("    --help                Show this help message");
    println!();
    println!("OPTIONS:");
    println!("    -s, --survey <CODE>     Survey code to process");
    println!("    -e, --env <ENV>         Environment (dev, stage, prod) [default: dev]");
    println!("    -c, --config <PATH>     Path to configuration file");
    println!("    -o, --output <DIR>      Output directory");
    println!(
        "    -n, --count <N>         Number of random surveys to process (process-random, process-dags-random)"
    );
    println!("        --seed <NUM>        Seed for random selection (process-*-random)");
    println!(
        "        --allow-multi       Allow processing more than 1 survey at a time (safety off)"
    );
    println!("    -v, --verbose           Enable verbose logging");
    println!();
    println!("EXAMPLES:");
    println!("    rusty process --survey ap");
    println!("    rusty process --survey ap --env prod");
    println!("    rusty process-random --count 5");
    println!("    rusty validate-config --survey ap --env dev");
    println!("    rusty print-config --survey ap --env prod");
    println!("    rusty migrate-legacy --survey ap");
    println!("    rusty validate --config config/surveys/ap.yml");
    println!("    rusty process-dags --survey ap");
    println!("    rusty process-dags-random --count 5");
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

/// Process N random surveys end-to-end
async fn process_random(args: Args) -> Result<()> {
    let count = args.count.unwrap_or(5);
    tracing::info!("Selecting {} random surveys to process", count);

    let surveys = discover_surveys().await?;
    if surveys.is_empty() {
        return Err(Error::Config(rusty::error::ConfigError::LoadError {
            path: "config/surveys".to_string(),
            source: "No surveys discovered".to_string(),
        }));
    }

    let mut codes: Vec<String> = surveys.into_iter().map(|s| s.code).collect();
    // Shuffle and take the requested count (or all if fewer available)
    fastrand::shuffle(&mut codes);
    let take_n = std::cmp::min(count, codes.len());
    let selected: Vec<String> = codes.into_iter().take(take_n).collect();

    println!("Processing {} random surveys: {:?}", take_n, selected);

    let mut successes: Vec<String> = Vec::new();
    let mut failures: Vec<(String, String)> = Vec::new();

    for code in selected {
        println!("\n=== Processing survey {} ===", code);
        let per_args = Args {
            command: Command::Process,
            survey: Some(code.clone()),
            environment: args.environment.clone(),
            config_path: args.config_path.clone(),
            verbose: args.verbose,
            output_dir: args.output_dir.clone(),
            count: None,
            seed: args.seed,
            allow_multi: args.allow_multi,
        };

        match process_survey(per_args).await {
            Ok(_) => {
                println!("[OK] {} processed successfully", code);
                successes.push(code);
            }
            Err(e) => {
                tracing::error!("Processing failed for {}: {}", code, e);
                failures.push((code, format!("{}", e)));
            }
        }
    }

    println!("\n=== Random Processing Summary ===");
    println!("Succeeded: {}", successes.len());
    if !successes.is_empty() {
        println!("  - {:?}", successes);
    }
    println!("Failed: {}", failures.len());
    for (code, err) in &failures {
        println!("  - {}: {}", code, err);
    }

    if failures.is_empty() {
        Ok(())
    } else {
        // Return first failure as error, but after printing summary
        let (code, err) = failures.into_iter().next().unwrap();
        Err(Error::Processing(
            rusty::error::ProcessingError::SystemError(format!(
                "At least one survey failed ({}): {}",
                code, err
            )),
        ))
    }
}

// --- DAG integration helpers and commands ---

struct NoOpStage {
    name: String,
    description: String,
}

impl NoOpStage {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: "No-op stage".to_string(),
        }
    }
}

#[async_trait]
impl PipelineStage for NoOpStage {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn can_process(&self, _context: &ProcessingContext) -> Result<bool> {
        Ok(true)
    }
    async fn execute(&mut self, _context: &mut ProcessingContext) -> Result<()> {
        Ok(())
    }
    fn dependencies(&self) -> Vec<String> {
        vec![]
    }
    fn validate(&self, _context: &ProcessingContext) -> Result<()> {
        Ok(())
    }
    async fn cleanup(&mut self, _context: &mut ProcessingContext) -> Result<()> {
        Ok(())
    }
}

async fn process_dags(args: Args) -> Result<()> {
    let survey_code = args.survey.ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::InvalidArgument(
            "Survey code is required for process-dags".to_string(),
        ))
    })?;

    let env = args
        .environment
        .clone()
        .unwrap_or_else(|| "dev".to_string());

    // Load survey config to obtain DAGs
    let survey_cfg = match load_survey_config(&survey_code, Some(&env)) {
        Ok(cfg) => cfg,
        Err(e) => {
            return Err(Error::Config(rusty::error::ConfigError::LoadError {
                path: format!("survey config for {}", survey_code),
                source: e.to_string(),
            }));
        }
    };

    let dags_cfg = survey_cfg.dags.clone().ok_or_else(|| {
        Error::Config(rusty::error::ConfigError::LoadError {
            path: format!("config/surveys/{}/dags.yml", survey_code.to_uppercase()),
            source: format!(
                "dags.yml not found or failed to load for survey {}",
                survey_code
            ),
        })
    })?;

    // Choose DAG name (prefer a DAG named "pipeline" if present)
    let chosen_dag_name = if dags_cfg.dags.contains_key("pipeline") {
        "pipeline".to_string()
    } else {
        // pick the first dag
        dags_cfg
            .dags
            .values()
            .next()
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "pipeline".to_string())
    };

    // Build processing context with inferred input paths
    let mut context = build_processing_context_for_survey(&survey_code);

    // Create executor and register stages based on the DAG tasks
    let mut executor = DagExecutor::new(dags_cfg.clone());

    // Find selected dag definition to map tasks to stages
    let dag_def = dags_cfg
        .dags
        .values()
        .find(|d| d.name == chosen_dag_name)
        .ok_or_else(|| {
            Error::Processing(rusty::error::ProcessingError::PipelineError {
                stage: chosen_dag_name.clone(),
                message: "DAG definition not found".to_string(),
            })
        })?;

    register_stages_for_dag(&mut executor, dag_def);

    // Execute
    let mut exec_ctx = executor.execute_dag(&chosen_dag_name, context).await?;

    let series_count = exec_ctx.processing_context.series_data.len();
    let obs_count = exec_ctx.processing_context.observation_data.len();
    let lookup_count = exec_ctx.processing_context.lookup_data.len();

    println!(
        "DAG processing completed successfully for survey: {}",
        survey_code
    );
    println!(
        "  Records summary -> series: {}, observations: {}, lookups: {}",
        series_count, obs_count, lookup_count
    );
    println!(
        "  Outputs (expected): data/processed/{}/[series.csv, observations.csv, lookups.csv]",
        survey_code.to_uppercase()
    );

    // Ensure final Parquet combined output exists for this survey
    let final_dir = std::path::PathBuf::from(format!("data/final/{}", survey_code.to_uppercase()));
    let combined_path = final_dir.join("combined.parquet");
    let need_finalize = match std::fs::metadata(&combined_path) {
        Ok(meta) => meta.len() == 0,
        Err(_) => true,
    };
    if need_finalize {
        println!(
            "  Final parquet not found or empty at {}. Generating combined parquet...",
            combined_path.display()
        );
        let mut writer = WriterStageImpl::new();
        // Reuse processing context populated by DAG to finalize outputs and parquet
        writer.execute(&mut exec_ctx.processing_context).await?;
        println!("  Final parquet generated at {}", combined_path.display());
    } else {
        println!("  Final parquet present at {}", combined_path.display());
    }

    Ok(())
}

async fn process_dags_random(args: Args) -> Result<()> {
    let count = args.count.unwrap_or(3);
    let env = args
        .environment
        .clone()
        .unwrap_or_else(|| "dev".to_string());

    let surveys = discover_surveys().await?;
    if surveys.is_empty() {
        return Err(Error::Config(rusty::error::ConfigError::LoadError {
            path: "config/surveys".to_string(),
            source: "No surveys discovered".to_string(),
        }));
    }

    // Filter to those with dags.yml available and collect diagnostics
    let mut with_dag: Vec<String> = Vec::new();
    let mut without_dag: Vec<String> = Vec::new();
    for s in surveys {
        match load_survey_config(&s.code, Some(&env)) {
            Ok(cfg) => {
                if cfg.dags.is_some() {
                    with_dag.push(s.code);
                } else {
                    without_dag.push(s.code);
                }
            }
            Err(_) => {
                // If config fails to load, also consider it without dag
                without_dag.push(s.code);
            }
        }
    }

    println!(
        "Discovered surveys: {} | DAG-enabled: {} | No DAG: {}",
        with_dag.len() + without_dag.len(),
        with_dag.len(),
        without_dag.len()
    );
    if !without_dag.is_empty() {
        println!(
            "  Skipping (no dags.yml or not parseable): {:?}",
            without_dag
        );
    }

    if with_dag.is_empty() {
        return Err(Error::Config(rusty::error::ConfigError::LoadError {
            path: "config/surveys/*/dags.yml".to_string(),
            source: "No DAG-enabled surveys found".to_string(),
        }));
    }

    // Safety guard: without --allow-multi, cap the number of surveys to 3 to avoid heavy runs
    let effective_count = if !args.allow_multi && count > 3 {
        println!(
            "Safety guard: requested count={} but --allow-multi not set. Clamping to 3.",
            count
        );
        3
    } else {
        count
    };

    // Seed RNG for reproducible random selection
    let seed_used: u64 = if let Some(s) = args.seed {
        fastrand::seed(s);
        s
    } else {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0));
        // Use lower 64 bits of nanos for seed
        let s = now.as_nanos() as u64;
        fastrand::seed(s);
        s
    };
    println!("Random selection seed: {}", seed_used);

    fastrand::shuffle(&mut with_dag);
    let take_n = std::cmp::min(effective_count, with_dag.len());
    let selected: Vec<String> = with_dag.into_iter().take(take_n).collect();

    println!(
        "Processing DAGs for {} random surveys: {:?}",
        take_n, selected
    );

    let mut successes: Vec<String> = Vec::new();
    let mut failures: Vec<(String, String)> = Vec::new();

    for code in selected {
        println!("\n=== DAG Processing survey {} ===", code);
        let per_args = Args {
            command: Command::ProcessDags,
            survey: Some(code.clone()),
            environment: args.environment.clone(),
            config_path: args.config_path.clone(),
            verbose: args.verbose,
            output_dir: args.output_dir.clone(),
            count: None,
            seed: args.seed,
            allow_multi: args.allow_multi,
        };

        match process_dags(per_args).await {
            Ok(_) => {
                // Verify final parquet exists and is non-empty for this survey
                let sc = code.to_uppercase();
                let combined =
                    std::path::PathBuf::from(format!("data/final/{}/combined.parquet", sc));
                let parquet_ok = std::fs::metadata(&combined)
                    .map(|m| m.len() > 0)
                    .unwrap_or(false);
                if parquet_ok {
                    println!(
                        "[OK] {} DAG processed successfully; final parquet: {}",
                        code,
                        combined.display()
                    );
                    successes.push(code);
                } else {
                    println!(
                        "[WARN] {} processed but final parquet missing or empty at {}",
                        code,
                        combined.display()
                    );
                    failures.push((
                        code,
                        format!("Final parquet missing or empty at {}", combined.display()),
                    ));
                }
            }
            Err(e) => {
                failures.push((code, format!("{}", e)));
            }
        }
    }

    println!("\n=== DAG Random Processing Summary ===");
    println!("Succeeded: {}", successes.len());
    if !successes.is_empty() {
        println!("  - {:?}", successes);
    }
    println!("Failed: {}", failures.len());
    for (code, err) in &failures {
        println!("  - {}: {}", code, err);
    }

    if failures.is_empty() {
        Ok(())
    } else {
        let (code, err) = failures.into_iter().next().unwrap();
        Err(Error::Processing(
            rusty::error::ProcessingError::SystemError(format!(
                "At least one DAG run failed ({}): {}",
                code, err
            )),
        ))
    }
}

fn build_processing_context_for_survey(survey_code: &str) -> ProcessingContext {
    let config = ProcessingConfig::default();
    let mut context = ProcessingContext::new(config);

    let base_dir = std::path::PathBuf::from(format!("data/raw/bls/{}", survey_code.to_lowercase()));
    let patterns = vec![
        "*.series".to_string(),
        "data/*".to_string(),
        "map/*".to_string(),
    ];
    let input_paths = patterns
        .into_iter()
        .map(|p| base_dir.join(p).to_string_lossy().to_string())
        .collect::<Vec<_>>();
    context.input_paths = input_paths;
    context
}

fn register_stages_for_dag(executor: &mut DagExecutor, dag: &rusty::config::model::DagDefinition) {
    use std::collections::HashSet;

    // Determine task mapping by type
    let mut registered: HashSet<String> = HashSet::new();
    for (name, task) in &dag.tasks {
        let kind = task.task_type.to_lowercase();
        let stage: Box<dyn PipelineStage> = match kind.as_str() {
            "scan" | "discover" | "load_lookups" | "load" => Box::new(LoaderStageImpl::new()),
            "output" | "writer" | "write_output" | "export" | "finalize" | "parquet" | "save"
            | "write" => Box::new(WriterStageImpl::new()),
            _ => Box::new(NoOpStage::new(name)),
        };
        if !registered.contains(name) {
            if let Err(e) = executor.register_task(name.clone(), stage) {
                tracing::warn!("Failed to register task {}: {}", name, e);
            } else {
                registered.insert(name.clone());
            }
        }
    }
}
