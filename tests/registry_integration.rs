use std::path::Path;

#[test]
fn registry_file_parses_and_has_expected_keys() {
    let path = Path::new("models/registry.json");
    assert!(path.exists());

    let data = std::fs::read_to_string(path).expect("read registry");
    let parsed: serde_json::Value = serde_json::from_str(&data).expect("valid json");

    let models = parsed
        .get("models")
        .and_then(|m| m.as_array())
        .expect("models array");

    assert!(!models.is_empty());
    for model in models {
        assert!(model.get("id").is_some());
        assert!(model.get("task").is_some());
        assert!(model.get("format").is_some());
        assert!(model.get("quantizations").is_some());
    }
}
