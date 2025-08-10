use rusty::survey::loader::UnifiedSurveyLoader;

#[test]
fn test_load_unified_survey() {
    // Create a loader
    let loader = UnifiedSurveyLoader::new();
    
    // Load the AP survey
    let result = loader.load_survey("ap");
    assert!(result.is_ok(), "Failed to load AP survey: {:?}", result.err());
    
    let survey = result.unwrap();
    assert_eq!(survey.metadata.code, "AP");
    assert_eq!(survey.metadata.name, "Average Price Data");
    assert!(survey.metadata.description.is_some());
    
    // Check that we have at least overview and model loaded
    let loaded_configs = survey.get_loaded_configs();
    assert!(loaded_configs.contains(&"overview".to_string()));
    assert!(loaded_configs.contains(&"model".to_string()));
    
    println!("Loaded configs: {:?}", loaded_configs);
}

#[test]
fn test_list_surveys() {
    let loader = UnifiedSurveyLoader::new();
    
    let result = loader.list_surveys();
    assert!(result.is_ok(), "Failed to list surveys: {:?}", result.err());
    
    let surveys = result.unwrap();
    assert!(!surveys.is_empty(), "No surveys found");
    assert!(surveys.contains(&"AP".to_string()), "AP survey not found");
    
    println!("Found {} surveys", surveys.len());
}