use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn recommend_contains_model_id() {
    let temp_home = std::env::temp_dir().join(format!(
        "tokensmith-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_secs()
    ));
    std::fs::create_dir_all(&temp_home).expect("create temp home");

    let profile = serde_json::json!({
        "os": "macos",
        "arch": "aarch64",
        "cpu_brand": "Test CPU",
        "logical_cores": 12,
        "physical_cores": 12,
        "performance_cores": 8,
        "efficiency_cores": 4,
        "total_mem_bytes": 68719476736u64,
        "available_mem_bytes": 42949672960u64,
        "has_gpu_accel": true,
        "gpu_backend": "metal"
    });

    let mut cmd = Command::cargo_bin("tokensmith").expect("binary exists");
    cmd.env("TOKENSMITH_HOME", &temp_home);
    cmd.env("TOKENSMITH_TEST_PROFILE_JSON", profile.to_string());
    cmd.args(["recommend", "--task", "code", "--mode", "fast"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Model:"));
}
