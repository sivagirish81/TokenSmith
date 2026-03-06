use anyhow::{anyhow, Result};

use crate::{
    cli::Task,
    model_registry::{ModelEntry, Registry},
    optimizer::{memory::MemoryConfig, profiles::Mode},
    profiler::types::HardwareProfile,
};

pub mod heuristics;

#[derive(Debug, Clone)]
pub struct Selection {
    pub model: ModelEntry,
    pub quantization: String,
    pub context_tokens: u32,
    pub threads: u32,
    pub reasons: Vec<String>,
}

pub fn recommend(
    registry: &Registry,
    hw: &HardwareProfile,
    task: Task,
    mode: Mode,
    cfg: &MemoryConfig,
) -> Result<Selection> {
    let candidates = heuristics::choose_model(registry, task);
    for c in candidates {
        if let Some((quant, ctx, mut reasons)) = heuristics::choose_quant_and_ctx(c, mode, hw, cfg)
        {
            let threads = if let Some(perf) = hw.performance_cores {
                perf
            } else {
                (hw.logical_cores.saturating_sub(2)).max(1).min(16)
            };
            reasons.push(format!("threads set to {}", threads));
            reasons.push(format!("mode {} selected", mode.as_str()));
            return Ok(Selection {
                model: c.clone(),
                quantization: quant,
                context_tokens: ctx,
                threads,
                reasons,
            });
        }
    }
    Err(anyhow!(
        "no model in registry fits memory for task '{}' and mode '{}'. try --mode fast",
        task,
        mode.as_str()
    ))
}
