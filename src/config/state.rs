use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{monitor::sample::MetricsSnapshot, optimizer::memory::MemoryConfig};

use super::paths::TokensmithPaths;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub optimizer: MemoryConfig,
}

impl AppConfig {
    pub fn load_or_default(paths: &TokensmithPaths) -> Result<Self> {
        let path = paths.config_path();
        if !path.exists() {
            let cfg = Self::default();
            cfg.save(path.as_path())?;
            return Ok(cfg);
        }
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerState {
    pub active: bool,
    pub pid: Option<u32>,
    pub started_at: u64,
    pub host: String,
    pub port: u16,
    pub task: String,
    pub mode: String,
    pub model_id: String,
    pub model_path: String,
    pub runtime: String,
    pub log_path: String,
    pub last_metrics: Option<MetricsSnapshot>,
    pub requests_served: Option<u64>,
    pub version: u32,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            active: false,
            pid: None,
            started_at: 0,
            host: "127.0.0.1".to_string(),
            port: 8000,
            task: "chat".to_string(),
            mode: "balanced".to_string(),
            model_id: "".to_string(),
            model_path: "".to_string(),
            runtime: "".to_string(),
            log_path: "".to_string(),
            last_metrics: None,
            requests_served: Some(0),
            version: 1,
        }
    }
}

impl ServerState {
    pub fn load(paths: &TokensmithPaths) -> Result<Self> {
        let path = paths.state_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn save(&self, paths: &TokensmithPaths) -> Result<()> {
        let path = paths.state_path();
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn active(
        pid: u32,
        host: &str,
        port: u16,
        task: &str,
        mode: &str,
        model_id: &str,
        model_path: &str,
        runtime: &str,
        log_path: &str,
    ) -> Self {
        Self {
            active: true,
            pid: Some(pid),
            started_at: crate::utils::time::unix_timestamp(),
            host: host.to_string(),
            port,
            task: task.to_string(),
            mode: mode.to_string(),
            model_id: model_id.to_string(),
            model_path: model_path.to_string(),
            runtime: runtime.to_string(),
            log_path: log_path.to_string(),
            last_metrics: None,
            requests_served: Some(0),
            version: 1,
        }
    }
}
