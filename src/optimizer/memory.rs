use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub safety_buffer_bytes: u64,
    pub kv_bytes_per_token_7b: u64,
    pub kv_bytes_per_token_14b: u64,
    pub overhead_bytes: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            safety_buffer_bytes: 4 * 1024 * 1024 * 1024,
            kv_bytes_per_token_7b: 1024 * 1024,
            kv_bytes_per_token_14b: 2 * 1024 * 1024,
            overhead_bytes: 1024 * 1024 * 1024,
        }
    }
}

pub fn usable_memory(total: u64, available: Option<u64>, cfg: &MemoryConfig) -> u64 {
    let total_safe = total.saturating_sub(cfg.safety_buffer_bytes);
    if let Some(avail) = available {
        avail.min(total_safe)
    } else {
        total_safe
    }
}

pub fn estimate_total_bytes(weights: u64, size_class: &str, ctx: u32, cfg: &MemoryConfig) -> u64 {
    let kv_per_token = if size_class.contains("14") {
        cfg.kv_bytes_per_token_14b
    } else {
        cfg.kv_bytes_per_token_7b
    };
    weights + (kv_per_token.saturating_mul(ctx as u64)) + cfg.overhead_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_and_no_fit() {
        let cfg = MemoryConfig::default();
        let usable = usable_memory(32 * 1024 * 1024 * 1024, Some(20 * 1024 * 1024 * 1024), &cfg);
        let fits = estimate_total_bytes(4 * 1024 * 1024 * 1024, "7b", 4096, &cfg) < usable;
        let no_fit = estimate_total_bytes(14 * 1024 * 1024 * 1024, "14b", 32768, &cfg) > usable;
        assert!(fits);
        assert!(no_fit);
    }
}
