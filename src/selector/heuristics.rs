use crate::{
    cli::Task,
    model_registry::{ModelEntry, Registry},
    optimizer::{memory, profiles::Mode},
    profiler::types::HardwareProfile,
};

pub fn choose_model<'a>(registry: &'a Registry, task: Task) -> Vec<&'a ModelEntry> {
    let task_str = task.to_string();
    let mut matches: Vec<&ModelEntry> = registry
        .models
        .iter()
        .filter(|m| m.task == task_str)
        .collect();
    matches.sort_by_key(|m| {
        if m.size_class.contains("14") {
            3
        } else if m.size_class.contains("8") {
            2
        } else {
            1
        }
    });
    matches.reverse();
    matches
}

pub fn choose_quant_and_ctx(
    model: &ModelEntry,
    mode: Mode,
    hw: &HardwareProfile,
    cfg: &memory::MemoryConfig,
) -> Option<(String, u32, Vec<String>)> {
    let usable = memory::usable_memory(hw.total_mem_bytes, hw.available_mem_bytes, cfg);
    let mut reasons = vec![format!(
        "usable memory budget: {:.2} GiB",
        usable as f64 / 1024_f64.powi(3)
    )];
    let mut context_candidates = mode.context_targets();
    for ctx in [1024_u32, 512_u32] {
        if !context_candidates.contains(&ctx) {
            context_candidates.push(ctx);
        }
    }

    for quant in mode.quantization_preferences() {
        if let Some(q) = model.quantizations.iter().find(|q| q.name == quant) {
            for ctx in &context_candidates {
                let est =
                    memory::estimate_total_bytes(q.approx_size_bytes, &model.size_class, *ctx, cfg);
                if est < usable {
                    reasons.push(format!("selected quant {} at context {}", quant, ctx));
                    return Some((quant.to_string(), *ctx, reasons));
                }
            }
        }
    }
    None
}
