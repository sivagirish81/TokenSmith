use anyhow::Result;
use sysinfo::System;

use super::types::HardwareProfile;

pub fn profile() -> Result<HardwareProfile> {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu_brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    Ok(HardwareProfile {
        os: "linux".to_string(),
        arch: std::env::consts::ARCH.to_string(),
        cpu_brand,
        logical_cores: sys.cpus().len() as u32,
        physical_cores: sys.physical_core_count().map(|v| v as u32),
        performance_cores: None,
        efficiency_cores: None,
        total_mem_bytes: sys.total_memory(),
        available_mem_bytes: Some(sys.available_memory()),
        has_gpu_accel: false,
        gpu_backend: Some("none".to_string()),
    })
}
