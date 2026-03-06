use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct TokensmithPaths {
    root: PathBuf,
}

impl TokensmithPaths {
    pub fn new() -> Result<Self> {
        if let Ok(home_override) = std::env::var("TOKENSMITH_HOME") {
            return Ok(Self {
                root: PathBuf::from(home_override),
            });
        }
        let home = dirs::home_dir().ok_or_else(|| anyhow!("could not resolve home directory"))?;
        Ok(Self {
            root: home.join(".tokensmith"),
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        std::fs::create_dir_all(self.root())?;
        std::fs::create_dir_all(self.models_dir())?;
        std::fs::create_dir_all(self.logs_dir())?;
        std::fs::create_dir_all(self.bin_dir())?;
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    pub fn model_dir(&self, id: &str) -> PathBuf {
        self.models_dir().join(id)
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    pub fn state_path(&self) -> PathBuf {
        self.root.join("state.json")
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn new_server_log_path(&self) -> PathBuf {
        let ts = crate::utils::time::unix_timestamp();
        self.logs_dir().join(format!("server-{ts}.log"))
    }
}
