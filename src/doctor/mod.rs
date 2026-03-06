use crate::profiler::types::HardwareProfile;

pub fn summarize(profile: &HardwareProfile) -> Vec<String> {
    vec![
        format!("{} {}", profile.os, profile.arch),
        format!("logical cores: {}", profile.logical_cores),
        format!(
            "gpu backend: {}",
            profile
                .gpu_backend
                .clone()
                .unwrap_or_else(|| "none".to_string())
        ),
    ]
}
