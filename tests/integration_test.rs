use rustrland::config::Config;

#[tokio::test]
async fn test_config_loading() {
    let config = Config::default();
    assert!(!config.pyprland.plugins.is_empty());
    assert!(config.pyprland.plugins.contains(&"scratchpads".to_string()));
}
