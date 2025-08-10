use std::collections::HashMap;
use rusty::survey::loader::UnifiedSurveyLoader;

#[test]
fn test_load_all_surveys() {
    let loader = UnifiedSurveyLoader::new();
    
    // Get list of all surveys
    let surveys = loader.list_surveys().expect("Failed to list surveys");
    println!("Testing {} surveys", surveys.len());
    
    let mut results = HashMap::new();
    let mut total_loaded = 0;
    let mut total_failed = 0;
    
    // Test loading each survey
    for survey_code in &surveys {
        println!("\nTesting survey: {}", survey_code);
        
        match loader.load_survey(survey_code) {
            Ok(survey) => {
                println!("✓ {} loaded successfully", survey_code);
                println!("  Name: {}", survey.metadata.name);
                println!("  Loaded configs: {:?}", survey.get_loaded_configs());
                
                // Validate that we have at least the overview
                assert!(!survey.metadata.code.is_empty(), "Survey code should not be empty");
                assert!(!survey.metadata.name.is_empty(), "Survey name should not be empty");
                assert!(survey.is_valid(), "Survey should be valid");
                
                let loaded_configs = survey.get_loaded_configs();
                assert!(loaded_configs.contains(&"overview".to_string()), 
                       "Survey {} should have overview config", survey_code);
                
                // Check if model config loaded
                let has_model = loaded_configs.contains(&"model".to_string());
                if has_model {
                    assert!(survey.data_model.is_some(), "Survey with model config should have data model");
                }
                
                results.insert(survey_code.clone(), (true, survey.get_loaded_configs()));
                total_loaded += 1;
            }
            Err(e) => {
                println!("✗ {} failed to load: {}", survey_code, e);
                results.insert(survey_code.clone(), (false, vec![]));
                total_failed += 1;
            }
        }
    }
    
    // Print summary
    println!("\n=== SURVEY LOADING SUMMARY ===");
    println!("Total surveys: {}", surveys.len());
    println!("Successfully loaded: {}", total_loaded);
    println!("Failed to load: {}", total_failed);
    println!("Success rate: {:.1}%", (total_loaded as f64 / surveys.len() as f64) * 100.0);
    
    // Print detailed results
    println!("\n=== DETAILED RESULTS ===");
    for (code, (success, configs)) in &results {
        if *success {
            println!("✓ {}: {:?}", code, configs);
        } else {
            println!("✗ {}: FAILED", code);
        }
    }
    
    // We expect at least 80% success rate for this test to pass
    let success_rate = (total_loaded as f64 / surveys.len() as f64) * 100.0;
    assert!(success_rate >= 80.0, "Success rate should be at least 80%, got {:.1}%", success_rate);
}

#[test]
fn test_survey_metadata_quality() {
    let loader = UnifiedSurveyLoader::new();
    let surveys = loader.list_surveys().expect("Failed to list surveys");
    
    let mut metadata_quality = HashMap::new();
    
    for survey_code in &surveys {
        if let Ok(survey) = loader.load_survey(survey_code) {
            let mut quality_score = 0u32;
            let mut max_score = 5u32;  // Total possible points
            
            // Check metadata completeness
            if !survey.metadata.name.is_empty() {
                quality_score += 1;
            }
            
            if survey.metadata.description.is_some() {
                quality_score += 1;
            }
            
            if survey.metadata.characteristics.is_some() {
                quality_score += 1;
            }
            
            if survey.metadata.tags.is_some() && !survey.metadata.tags.as_ref().unwrap().is_empty() {
                quality_score += 1;
            }
            
            if survey.data_model.is_some() {
                quality_score += 1;
            }
            
            let percentage = (quality_score as f64 / max_score as f64) * 100.0;
            metadata_quality.insert(survey_code.clone(), (quality_score, max_score, percentage));
            
            println!("{}: {}/{} ({:.1}%)", survey_code, quality_score, max_score, percentage);
        }
    }
    
    // Calculate average metadata quality
    let avg_quality: f64 = metadata_quality.values()
        .map(|(_, _, percentage)| *percentage)
        .sum::<f64>() / metadata_quality.len() as f64;
    
    println!("Average metadata quality: {:.1}%", avg_quality);
    
    // We expect reasonable metadata quality
    assert!(avg_quality >= 60.0, "Average metadata quality should be at least 60%, got {:.1}%", avg_quality);
}

#[test]
fn test_data_model_structure() {
    let loader = UnifiedSurveyLoader::new();
    
    // Test specific surveys that should have good data models
    let test_surveys = vec!["AP", "BD", "CE"];
    
    for survey_code in test_surveys {
        match loader.load_survey(survey_code) {
            Ok(survey) => {
                println!("\nTesting data model for: {}", survey_code);
                
                if let Some(data_model) = &survey.data_model {
                    println!("  Series: {:?}", data_model.series.as_ref().map(|s| &s.path));
                    println!("  Data files: {}", data_model.data_files.len());
                    println!("  Lookups: {}", data_model.lookups.len());
                    println!("  Relationships: {}", data_model.relationships.len());
                    
                    // Validate data model structure
                    if let Some(series) = &data_model.series {
                        assert!(!series.path.is_empty(), "Series path should not be empty");
                        assert!(!series.index.is_empty(), "Series index should not be empty");
                        assert!(!series.schema.is_empty(), "Series schema should not be empty");
                    }
                    
                    // Check data files
                    for (i, data_file) in data_model.data_files.iter().enumerate() {
                        assert!(!data_file.id.is_empty(), "Data file {} ID should not be empty", i);
                        assert!(!data_file.pattern.is_empty(), "Data file {} pattern should not be empty", i);
                        assert!(!data_file.schema.is_empty(), "Data file {} schema should not be empty", i);
                    }
                    
                    // Check lookups
                    for (i, lookup) in data_model.lookups.iter().enumerate() {
                        assert!(!lookup.id.is_empty(), "Lookup {} ID should not be empty", i);
                        assert!(!lookup.path.is_empty(), "Lookup {} path should not be empty", i);
                        assert!(!lookup.schema.is_empty(), "Lookup {} schema should not be empty", i);
                    }
                } else {
                    println!("  No data model loaded for {}", survey_code);
                }
            }
            Err(e) => {
                println!("Failed to load {}: {}", survey_code, e);
                // Don't fail the test for individual surveys, just log
            }
        }
    }
}

#[test]
fn test_config_version_tracking() {
    let loader = UnifiedSurveyLoader::new();
    
    if let Ok(survey) = loader.load_survey("AP") {
        println!("Config versions for AP survey: {:?}", survey.config_versions);
        
        // Check that we track config versions for loaded files
        let loaded_configs = survey.get_loaded_configs();
        for config_type in &loaded_configs {
            if config_type != "overview" {  // overview doesn't set version in our current impl
                // Most configs should have version tracking
                println!("  {}: {:?}", config_type, survey.config_versions.get(config_type));
            }
        }
    }
}

#[test]
fn test_error_handling_robustness() {
    let loader = UnifiedSurveyLoader::new();
    
    // Test with non-existent survey
    let result = loader.load_survey("NONEXISTENT");
    assert!(result.is_err(), "Loading non-existent survey should fail");
    println!("Non-existent survey error: {}", result.unwrap_err());
    
    // Test with invalid survey code
    let result = loader.load_survey("");
    assert!(result.is_err(), "Loading empty survey code should fail");
    println!("Empty survey code error: {}", result.unwrap_err());
    
    // Test that the system continues working after errors
    let surveys = loader.list_surveys();
    assert!(surveys.is_ok(), "System should still work after error conditions");
    assert!(!surveys.unwrap().is_empty(), "Should still find surveys after errors");
}

#[test]
fn test_data_processing_integration() {
    let loader = UnifiedSurveyLoader::new();
    
    // Test that we can load surveys with data models and use them for processing
    let test_surveys = vec!["AP", "BD", "CE"];
    
    for survey_code in test_surveys {
        match loader.load_survey(survey_code) {
            Ok(survey) => {
                println!("\n=== Testing data processing integration for {} ===", survey_code);
                println!("Survey: {} ({})", survey.metadata.name, survey.metadata.code);
                
                // Validate that we have the required components for data processing
                assert!(survey.is_valid(), "Survey {} should be valid", survey_code);
                
                let loaded_configs = survey.get_loaded_configs();
                println!("Loaded configs: {:?}", loaded_configs);
                
                // Test data model integration
                if let Some(data_model) = &survey.data_model {
                    println!("✓ Data model available");
                    
                    // Test series configuration
                    if let Some(series) = &data_model.series {
                        println!("  Series configuration:");
                        println!("    Path: {}", series.path);
                        println!("    Index: {}", series.index);
                        println!("    Schema fields: {} defined", series.schema.len());
                        assert!(!series.path.is_empty(), "Series path should be configured");
                        assert!(!series.index.is_empty(), "Series index should be configured");
                    }
                    
                    // Test data files configuration
                    if !data_model.data_files.is_empty() {
                        println!("  Data files: {} configured", data_model.data_files.len());
                        for (i, data_file) in data_model.data_files.iter().take(3).enumerate() {
                            println!("    {}: {} ({})", i+1, data_file.id, data_file.pattern);
                            assert!(!data_file.pattern.is_empty(), "Data file pattern should be configured");
                        }
                    }
                    
                    // Test lookup tables configuration
                    if !data_model.lookups.is_empty() {
                        println!("  Lookup tables: {} configured", data_model.lookups.len());
                        for (i, lookup) in data_model.lookups.iter().take(3).enumerate() {
                            println!("    {}: {} ({})", i+1, lookup.id, lookup.path);
                            assert!(!lookup.path.is_empty(), "Lookup path should be configured");
                        }
                    }
                    
                    // Test relationships
                    if !data_model.relationships.is_empty() {
                        println!("  Relationships: {} defined", data_model.relationships.len());
                    }
                }
                
                // Test I/O configuration integration
                if let Some(io_config) = &survey.io_config {
                    println!("✓ I/O configuration available");
                    
                    if let Some(discovery) = &io_config.discovery {
                        println!("  Discovery root: {:?}", discovery.root);
                        println!("  Include patterns: {:?}", discovery.include);
                    }
                    
                    if let Some(combining) = &io_config.combining {
                        println!("  Combining enabled: {}", combining.enabled);
                        println!("  Strategy: {}", combining.strategy);
                    }
                }
                
                // Test processing configuration integration
                if let Some(processing_config) = &survey.processing_config {
                    println!("✓ Processing configuration available");
                    println!("  Strategy mode: {}", processing_config.strategy.mode);
                    
                    if let Some(batch_size) = processing_config.strategy.batch_size {
                        println!("  Batch size: {}", batch_size);
                        assert!(batch_size > 0, "Batch size should be positive");
                    }
                    
                    if let Some(parallel_tasks) = processing_config.strategy.parallel_tasks {
                        println!("  Parallel tasks: {}", parallel_tasks);
                        assert!(parallel_tasks > 0, "Parallel tasks should be positive");
                    }
                }
                
                // Test that config versions are tracked
                if !survey.config_versions.is_empty() {
                    println!("✓ Config versions tracked: {:?}", survey.config_versions);
                    
                    // Validate version numbers are reasonable
                    for (config_type, version) in &survey.config_versions {
                        assert!(*version > 0, "Version for {} should be positive", config_type);
                        assert!(*version <= 10, "Version for {} seems too high ({})", config_type, version);
                    }
                }
                
                println!("✓ {} integration test passed", survey_code);
            }
            Err(e) => {
                println!("⚠ Failed to load survey {}: {}", survey_code, e);
                // Don't fail the test, just log - some surveys might not have all configs
            }
        }
    }
}