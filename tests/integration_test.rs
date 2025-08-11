use rustrland::{Config, config::PyprlandConfig};
use std::collections::HashMap;
use tempfile::NamedTempFile;
use std::io::Write;

#[tokio::test]
async fn test_config_default() {
    let config = Config::default();
    assert!(!config.pyprland.plugins.is_empty());
    assert!(config.pyprland.plugins.contains(&"scratchpads".to_string()));
    assert!(config.plugins.is_empty());
}

#[tokio::test]
async fn test_config_from_file() {
    let config_content = r#"
[pyprland]
plugins = ["scratchpads", "expose"]

[scratchpads.term]
command = "kitty --class terminal"
class = "terminal"
size = "80% 60%"
animation = "fromTop"
"#;

    // Create temporary config file
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file.write_all(config_content.as_bytes()).expect("Failed to write to temp file");
    let temp_path = temp_file.path().to_str().unwrap();

    // Load config from file
    let config = Config::load(temp_path).await.expect("Failed to load config");
    
    // Verify basic structure
    assert_eq!(config.pyprland.plugins.len(), 2);
    assert!(config.pyprland.plugins.contains(&"scratchpads".to_string()));
    assert!(config.pyprland.plugins.contains(&"expose".to_string()));
    
    // Verify scratchpad config is parsed
    assert!(config.plugins.contains_key("scratchpads"));
}

#[test]
fn test_pyprland_config_creation() {
    let mut variables = HashMap::new();
    variables.insert("test_var".to_string(), "test_value".to_string());
    
    let pyprland_config = PyprlandConfig {
        plugins: vec!["scratchpads".to_string()],
        variables,
    };
    
    assert_eq!(pyprland_config.plugins.len(), 1);
    assert_eq!(pyprland_config.variables.get("test_var"), Some(&"test_value".to_string()));
}
