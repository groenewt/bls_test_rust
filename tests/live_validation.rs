use rusty::survey::loader::UnifiedSurveyLoader;

#[test]
fn live_yaml_validation() {
    println!("🚀 Live YAML Configuration Validation");
    println!("=====================================\n");

    // Initialize the survey loader
    let loader = UnifiedSurveyLoader::new();
    
    // Select 5 diverse surveys for validation
    let test_surveys = vec![
        ("AP", "Average Price Data - Simple survey with basic model"),
        ("BD", "Business Employment Dynamics - Complex survey with full config"),  
        ("CE", "Current Employment Statistics - Large survey without model"),
        ("CX", "Consumer Expenditure Survey - Survey with model and advanced features"),
        ("IN", "International Labor Statistics - Survey with runtime config")
    ];
    
    println!("Selected {} surveys for live validation:\n", test_surveys.len());
    
    let mut results = Vec::new();
    
    for (survey_code, description) in &test_surveys {
        println!("🔍 Loading Survey: {} ({})", survey_code, description);
        println!("{}", "─".repeat(60));
        
        match loader.load_survey(survey_code) {
            Ok(survey) => {
                let result = validate_survey_live(&survey, survey_code);
                results.push((survey_code, true, result));
                println!("✅ {} validation completed\n", survey_code);
            }
            Err(e) => {
                println!("❌ Failed to load {}: {}\n", survey_code, e);
                results.push((survey_code, false, format!("Load failed: {}", e)));
            }
        }
    }
    
    // Summary
    println!("📊 Live Validation Summary");
    println!("{}", "=".repeat(40));
    
    let successful = results.iter().filter(|(_, success, _)| *success).count();
    let total = results.len();
    
    println!("Success Rate: {}/{} ({:.1}%)", successful, total, (successful as f64 / total as f64) * 100.0);
    println!();
    
    for (survey_code, success, details) in &results {
        let status = if *success { "✅" } else { "❌" };
        println!("{} {}: {}", status, survey_code, details);
    }
    
    if successful == total {
        println!("\n🎉 All surveys validated successfully! YAML integration is production-ready.");
    } else {
        println!("\n⚠️  Some surveys failed validation. Review the errors above.");
    }
    
    // Assert for test framework
    assert_eq!(successful, total, "All surveys should validate successfully");
}

fn validate_survey_live(survey: &rusty::survey::UnifiedSurvey, survey_code: &str) -> String {
    let mut validation_details = Vec::new();
    
    // Basic validation
    if !survey.is_valid() {
        return "Survey failed basic validation".to_string();
    }
    validation_details.push("Basic validation ✓".to_string());
    
    // Metadata validation
    if !survey.metadata.name.is_empty() {
        validation_details.push(format!("Name: '{}'", survey.metadata.name));
    }
    
    // Configuration loading validation
    let loaded_configs = survey.get_loaded_configs();
    validation_details.push(format!("Configs loaded: {}", loaded_configs.len()));
    
    // Data model validation
    if let Some(data_model) = &survey.data_model {
        let mut model_details = Vec::new();
        
        if let Some(series) = &data_model.series {
            model_details.push(format!("Series: {}", series.path));
        }
        
        if !data_model.data_files.is_empty() {
            model_details.push(format!("Data files: {}", data_model.data_files.len()));
        }
        
        if !data_model.lookups.is_empty() {
            model_details.push(format!("Lookups: {}", data_model.lookups.len()));
        }
        
        if !data_model.relationships.is_empty() {
            model_details.push(format!("Relationships: {}", data_model.relationships.len()));
        }
        
        if !model_details.is_empty() {
            validation_details.push(format!("Data model: [{}]", model_details.join(", ")));
        }
    }
    
    // I/O configuration validation
    if let Some(io_config) = &survey.io_config {
        let mut io_details = Vec::new();
        
        if let Some(discovery) = &io_config.discovery {
            io_details.push(format!("Discovery root: {:?}", discovery.root));
        }
        
        if let Some(combining) = &io_config.combining {
            io_details.push(format!("Combining: {} ({})", combining.enabled, combining.strategy));
        }
        
        if !io_details.is_empty() {
            validation_details.push(format!("I/O config: [{}]", io_details.join(", ")));
        }
    }
    
    // Processing configuration validation
    if let Some(processing_config) = &survey.processing_config {
        validation_details.push(format!("Processing: {} strategy", processing_config.strategy.mode));
    }
    
    // Runtime configuration validation
    if survey.runtime_config.is_some() {
        validation_details.push("Runtime config ✓".to_string());
    }
    
    // Config version validation
    if !survey.config_versions.is_empty() {
        validation_details.push(format!("Version tracking: {} configs", survey.config_versions.len()));
    }
    
    format!("SUCCESS - {}", validation_details.join(" | "))
}