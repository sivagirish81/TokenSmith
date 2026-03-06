use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub task: String,
    pub size_class: String,
    pub format: String,
    pub downloads: Vec<String>,
    pub sha256: Option<String>,
    pub quantizations: Vec<QuantizationInfo>,
    pub default_context: u32,
    pub max_context: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantizationInfo {
    pub name: String,
    pub approx_size_bytes: u64,
}
