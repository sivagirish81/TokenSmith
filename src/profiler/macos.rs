use std::process::Command;

use anyhow::Result;
use sysinfo::System;

use super::types::HardwareProfile;

fn sysctl_value(key: &str) -> Option<String> {
    let out = Command::new("sysctl").arg("-n").arg(key).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn parse_vm_stat_pages(text: &str, key: &str) -> u64 {
    text.lines()
        .find(|l| l.trim_start().starts_with(key))
        .and_then(|l| l.split(':').nth(1))
        .map(|v| v.chars().filter(|c| c.is_ascii_digit()).collect::<String>())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
}

pub fn profile() -> Result<HardwareProfile> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_brand = sysctl_value("machdep.cpu.brand_string").unwrap_or_else(|| {
        sys.cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string())
    });
    let logical_cores = sysctl_value("hw.logicalcpu")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or_else(|| sys.cpus().len().max(1) as u32);
    let physical_cores = sysctl_value("hw.physicalcpu")
        .and_then(|v| v.parse::<u32>().ok())
        .or_else(|| sys.physical_core_count().map(|v| v as u32));
    let perf = sysctl_value("hw.perflevel0.physicalcpu").and_then(|v| v.parse::<u32>().ok());
    let eff = sysctl_value("hw.perflevel1.physicalcpu").and_then(|v| v.parse::<u32>().ok());
    let total_mem_bytes = sysctl_value("hw.memsize")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or_else(|| sys.total_memory());

    let available_mem_bytes = Command::new("vm_stat").output().ok().map(|o| {
        let text = String::from_utf8_lossy(&o.stdout);
        let page_size = text
            .lines()
            .find(|l| l.contains("page size of"))
            .and_then(|l| l.split("page size of ").nth(1))
            .and_then(|s| s.split(" bytes").next())
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(4096);
        let free_pages = parse_vm_stat_pages(&text, "Pages free:");
        let inactive_pages = parse_vm_stat_pages(&text, "Pages inactive:");
        let speculative_pages = parse_vm_stat_pages(&text, "Pages speculative:");
        let purgeable_pages = parse_vm_stat_pages(&text, "Pages purgeable:");
        let reclaimable_pages = free_pages + inactive_pages + speculative_pages + purgeable_pages;
        reclaimable_pages * page_size
    });

    let arch = std::env::consts::ARCH.to_string();
    let apple_silicon = arch == "aarch64" || arch == "arm64";

    Ok(HardwareProfile {
        os: "macos".to_string(),
        arch,
        cpu_brand,
        logical_cores,
        physical_cores,
        performance_cores: perf,
        efficiency_cores: eff,
        total_mem_bytes,
        available_mem_bytes,
        has_gpu_accel: apple_silicon,
        gpu_backend: Some(if apple_silicon { "metal" } else { "none" }.to_string()),
    })
}
