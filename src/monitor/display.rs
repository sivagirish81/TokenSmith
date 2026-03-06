use crate::config::state::ServerState;

use super::sample::MetricsSnapshot;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";

pub fn print_status_snapshot(state: &ServerState, snapshot: Option<&MetricsSnapshot>) {
    println!("Active: {}", state.active);
    println!("PID: {:?}", state.pid);
    println!("Bind: {}:{}", state.host, state.port);
    println!("Task/Mode: {}/{}", state.task, state.mode);
    println!("Model: {}", state.model_id);
    println!("Runtime: {}", state.runtime);

    if let Some(s) = snapshot {
        println!("RSS (MB): {:.2}", s.rss_bytes as f64 / (1024.0 * 1024.0));
        println!("CPU (%): {:.2}", s.cpu_percent);
        println!("Threads: {}", s.threads);
        println!("Uptime (s): {}", s.uptime_secs);
    }
}

pub fn print_monitor_frame(
    state: &ServerState,
    snapshot: &MetricsSnapshot,
    watch: bool,
    warn_mem: Option<f32>,
    warn_cpu: Option<f32>,
) {
    if watch {
        print!("\x1B[2J\x1B[1;1H");
    }

    let mem_pct = snapshot
        .total_mem_bytes
        .map(|total| (snapshot.rss_bytes as f32 / total as f32) * 100.0);
    let cpu_pct = snapshot.cpu_percent;
    let mem_warn = warn_mem.unwrap_or(80.0);
    let cpu_warn = warn_cpu.unwrap_or(300.0);

    println!("{BOLD}{CYAN}tokensmith monitor{RESET}");
    println!(
        "model={} mode={} bind={}:{} pid={}",
        state.model_id,
        state.mode,
        state.host,
        state.port,
        state.pid.unwrap_or_default()
    );

    let rss_mb = snapshot.rss_bytes as f64 / (1024.0 * 1024.0);
    println!(
        "rss_mb={:.2}  threads={}  uptime={}s",
        rss_mb, snapshot.threads, snapshot.uptime_secs
    );
    println!(
        "memory: {} {:.1}% (warn {:.1}%)",
        usage_bar(mem_pct.unwrap_or(0.0), mem_warn, 40),
        mem_pct.unwrap_or(0.0),
        mem_warn
    );
    println!(
        "cpu:    {} {:.1}% (warn {:.1}%)",
        usage_bar(cpu_pct, cpu_warn, 40),
        cpu_pct,
        cpu_warn
    );

    if let (Some(total), Some(free)) = (snapshot.total_mem_bytes, snapshot.available_mem_bytes) {
        println!(
            "system: total={:.2} GiB free={:.2} GiB",
            total as f64 / 1024_f64.powi(3),
            free as f64 / 1024_f64.powi(3)
        );
    }

    if let Some(pct) = mem_pct {
        if pct > mem_warn {
            println!(
                "{RED}{BOLD}warning:{RESET}{RED} memory {:.1}% exceeds {:.1}%  -> try `tokensmith throttle --mode fast` or `tokensmith stop`{RESET}",
                pct, mem_warn
            );
        }
    }
    if cpu_pct > cpu_warn {
        println!(
            "{RED}{BOLD}warning:{RESET}{RED} cpu {:.1}% exceeds {:.1}%  -> try `tokensmith throttle --mode fast` or `tokensmith stop`{RESET}",
            cpu_pct, cpu_warn
        );
    }
}

fn usage_bar(value_pct: f32, warn_pct: f32, width: usize) -> String {
    let ratio = if warn_pct <= 0.0 {
        0.0
    } else {
        (value_pct / warn_pct).max(0.0)
    };
    let fill = ((ratio.min(1.0)) * width as f32).round() as usize;
    let mut bar = String::with_capacity(width);
    for i in 0..width {
        if i < fill {
            bar.push('#');
        } else {
            bar.push('-');
        }
    }
    let color = if value_pct >= warn_pct {
        RED
    } else if value_pct >= warn_pct * 0.7 {
        YELLOW
    } else {
        GREEN
    };
    format!("{color}[{bar}]{RESET}")
}
