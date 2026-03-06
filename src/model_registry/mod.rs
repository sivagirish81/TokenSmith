use anyhow::{anyhow, Result};

pub mod types;

pub use types::{ModelEntry, Registry};

impl Registry {
    pub fn load_and_validate(path: &str) -> Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let registry: Registry = serde_json::from_str(&data)?;
        registry.validate()?;
        Ok(registry)
    }

    pub fn validate(&self) -> Result<()> {
        if self.models.is_empty() {
            return Err(anyhow!("registry has no models"));
        }
        for m in &self.models {
            if m.id.trim().is_empty() {
                return Err(anyhow!("model id is empty"));
            }
            if m.format != "gguf" {
                return Err(anyhow!(
                    "model {} has unsupported format {}",
                    m.id,
                    m.format
                ));
            }
            if m.downloads.is_empty() {
                return Err(anyhow!("model {} has no downloads", m.id));
            }
            if m.quantizations.is_empty() {
                return Err(anyhow!("model {} has no quantization entries", m.id));
            }
        }
        Ok(())
    }

    pub fn by_id(&self, id: &str) -> Option<&ModelEntry> {
        self.models.iter().find(|m| m.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_registry() {
        let reg =
            Registry::load_and_validate("models/registry.json").expect("registry should parse");
        assert!(!reg.models.is_empty());
    }
}
