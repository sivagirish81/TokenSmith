use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessesToUpdate, System};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub rss_bytes: u64,
    pub cpu_percent: f32,
    pub threads: usize,
    pub uptime_secs: u64,
    pub total_mem_bytes: Option<u64>,
    pub available_mem_bytes: Option<u64>,
}

pub trait MetricSampler {
    fn sample(&self, pid: u32, started_at: u64) -> Result<MetricsSnapshot>;
}

pub struct DefaultSampler;

pub fn default_sampler() -> DefaultSampler {
    DefaultSampler
}

impl MetricSampler for DefaultSampler {
    fn sample(&self, pid: u32, started_at: u64) -> Result<MetricsSnapshot> {
        let mut sys = System::new_all();
        sys.refresh_processes(ProcessesToUpdate::All, true);
        sys.refresh_memory();

        if let Some(proc_) = sys.process(Pid::from_u32(pid)) {
            #[cfg(target_os = "macos")]
            let mut cpu = proc_.cpu_usage();
            #[cfg(not(target_os = "macos"))]
            let cpu = proc_.cpu_usage();

            #[cfg(target_os = "macos")]
            let mut rss_bytes = proc_.memory();
            #[cfg(not(target_os = "macos"))]
            let rss_bytes = proc_.memory();

            #[cfg(target_os = "macos")]
            {
                if cpu <= 0.0 {
                    if let Some(v) = fallback_ps_cpu(pid) {
                        cpu = v;
                    }
                }
                if rss_bytes == 0 {
                    if let Some(v) = fallback_ps_rss_bytes(pid) {
                        rss_bytes = v;
                    }
                }
            }

            let now = crate::utils::time::unix_timestamp();
            return Ok(MetricsSnapshot {
                rss_bytes,
                cpu_percent: cpu,
                threads: proc_.tasks().map(|t| t.len()).unwrap_or(1),
                uptime_secs: now.saturating_sub(started_at),
                total_mem_bytes: Some(sys.total_memory()),
                available_mem_bytes: Some(sys.available_memory()),
            });
        }

        Err(anyhow!("process {} not found", pid))
    }
}

#[cfg(target_os = "macos")]
fn fallback_ps_cpu(pid: u32) -> Option<f32> {
    let out = std::process::Command::new("ps")
        .args(["-o", "%cpu=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let v = String::from_utf8_lossy(&out.stdout);
    v.trim().parse::<f32>().ok()
}

#[cfg(target_os = "macos")]
fn fallback_ps_rss_bytes(pid: u32) -> Option<u64> {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let kb = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<u64>()
        .ok()?;
    Some(kb * 1024)
}
