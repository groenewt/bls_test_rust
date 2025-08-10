use std::path::PathBuf;

#[test]
fn test_yaml_adapter_load_lookup_ids_ap() {
    let path = PathBuf::from("config/surveys/AP/model.yml");
    let ids = rusty::config::yaml_adapter::load_yaml_lookup_ids(&path)
        .expect("failed to load lookup ids");
    assert_eq!(ids, vec![
        "area".to_string(),
        "footnote".to_string(),
        "item".to_string(),
        "period".to_string(),
    ]);
}
