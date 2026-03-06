use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub os: String,
    pub arch: String,
    pub cpu_brand: String,
    pub logical_cores: u32,
    pub physical_cores: Option<u32>,
    pub performance_cores: Option<u32>,
    pub efficiency_cores: Option<u32>,
    pub total_mem_bytes: u64,
    pub available_mem_bytes: Option<u64>,
    pub has_gpu_accel: bool,
    pub gpu_backend: Option<String>,
}
