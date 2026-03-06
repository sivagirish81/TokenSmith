mod cli;
mod config;
mod doctor;
mod model_registry;
mod monitor;
mod optimizer;
mod profiler;
mod runtime;
mod selector;
mod server;
mod utils;

use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cli::{Cli, Commands, InstallTarget};
use config::paths::TokensmithPaths;
use config::state::{AppConfig, ServerState};
use model_registry::Registry;
use monitor::display::{print_monitor_frame, print_status_snapshot};
use monitor::sample::MetricSampler;
use optimizer::profiles::Mode;
use runtime::process::{is_pid_alive, terminate_pid};
use selector::recommend;
use utils::download::download_with_progress;
use utils::logging::{init_background_logging, init_logging};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logging(cli.verbose)?;

    let paths = TokensmithPaths::new()?;
    paths.ensure_dirs()?;
    let config = AppConfig::load_or_default(&paths)?;
    let registry = Registry::load_and_validate("models/registry.json")?;

    match cli.command {
        Commands::Doctor => cmd_doctor(&paths).await?,
        Commands::Recommend { task, mode } => {
            let profile = profiler::profile_hardware()?;
            let selection = recommend(&registry, &profile, task, mode, &config.optimizer)?;
            println!("Model: {}", selection.model.id);
            println!("Quantization: {}", selection.quantization);
            println!("Context: {}", selection.context_tokens);
            println!("Threads: {}", selection.threads);
            for reason in selection.reasons {
                println!("Reason: {}", reason);
            }
        }
        Commands::Pull { model_id } => {
            let model = registry
                .by_id(&model_id)
                .ok_or_else(|| anyhow!("model '{model_id}' not found in registry"))?;
            let dest_dir = paths.model_dir(&model.id);
            std::fs::create_dir_all(&dest_dir)?;
            let filename = format!("{}.gguf", model.id);
            let dest = dest_dir.join(filename);
            let url = model
                .downloads
                .first()
                .ok_or_else(|| anyhow!("no download URL configured for {}", model.id))?;
            download_with_progress(url, &dest).await?;
            if let Some(expected) = &model.sha256 {
                let actual = utils::checksum::sha256_file(&dest)?;
                if &actual != expected {
                    return Err(anyhow!(
                        "checksum mismatch for {}: expected {}, got {}",
                        model.id,
                        expected,
                        actual
                    ));
                }
                println!("Checksum verified: {}", expected);
            } else {
                let actual = utils::checksum::sha256_file(&dest)?;
                println!(
                    "Warning: no SHA256 in registry. Computed SHA256: {}",
                    actual
                );
            }
            println!("Model saved to {}", dest.display());
        }
        Commands::Up {
            task,
            mode,
            ctx,
            port,
            host,
            detach,
        } => {
            cmd_up(
                &paths, &config, &registry, task, mode, ctx, host, port, detach,
            )
            .await?;
        }
        Commands::Status => cmd_status(&paths)?,
        Commands::Monitor {
            interval,
            watch,
            json,
            warn_mem,
            warn_cpu,
        } => cmd_monitor(&paths, interval, watch, json, warn_mem, warn_cpu).await?,
        Commands::Stop { force_after } => {
            cmd_stop(&paths, force_after).await?;
        }
        Commands::Kill => {
            cmd_stop(&paths, Duration::from_secs(0)).await?;
        }
        Commands::Throttle { mode } => {
            cmd_throttle(&paths, &config, &registry, mode).await?;
        }
        Commands::Ps => cmd_ps(&paths)?,
        Commands::Logs { follow, calls } => cmd_logs(&paths, follow, calls).await?,
        Commands::Install { target } => cmd_install(&paths, target)?,
        Commands::Serve {
            task,
            mode,
            host,
            port,
            model_id,
            model_path,
            runtime_url,
        } => {
            init_background_logging(&paths.logs_dir())?;
            server::run_server(task, mode, host, port, model_id, model_path, runtime_url).await?;
        }
    }

    Ok(())
}

async fn cmd_doctor(paths: &TokensmithPaths) -> Result<()> {
    let profile = profiler::profile_hardware()?;
    let llama_path = runtime::llama_cpp::find_llama_server(paths);

    println!("Hardware profile");
    println!("  OS: {}", profile.os);
    println!("  Arch: {}", profile.arch);
    println!("  CPU: {}", profile.cpu_brand);
    println!("  Logical cores: {}", profile.logical_cores);
    println!("  Performance cores: {:?}", profile.performance_cores);
    println!("  Efficiency cores: {:?}", profile.efficiency_cores);
    println!(
        "  Total memory (GiB): {:.2}",
        profile.total_mem_bytes as f64 / 1024_f64.powi(3)
    );
    println!(
        "  Available memory (GiB): {:?}",
        profile
            .available_mem_bytes
            .map(|m| format!("{:.2}", m as f64 / 1024_f64.powi(3)))
    );
    println!("  GPU accel: {}", profile.has_gpu_accel);
    println!("  GPU backend: {:?}", profile.gpu_backend);
    for line in doctor::summarize(&profile) {
        println!("  Summary: {line}");
    }

    match llama_path {
        Some(path) => println!("llama-server: found at {}", path.display()),
        None => {
            println!("llama-server: not found");
            println!(
                "Install guidance: place binary at ~/.tokensmith/bin/llama-server or add to PATH."
            );
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn cmd_up(
    paths: &TokensmithPaths,
    config: &AppConfig,
    registry: &Registry,
    task: cli::Task,
    mode: Mode,
    ctx: Option<u32>,
    host: String,
    port: u16,
    detach: bool,
) -> Result<()> {
    if !utils::net::port_available(&host, port) {
        return Err(anyhow!("port {} is already in use on {}", port, host));
    }

    let profile = profiler::profile_hardware()?;
    let (mut selection, model_path) = match recommend(
        registry,
        &profile,
        task,
        mode,
        &config.optimizer,
    ) {
        Ok(recommended) => {
            let recommended_model_id = recommended.model.id.clone();
            let (selection, model_path) = resolve_selection_with_local_fallback(
                paths,
                registry,
                &profile,
                task,
                mode,
                config,
                recommended,
            )?;
            if selection.model.id != recommended_model_id {
                println!(
                    "Warning: recommended model '{}' is not present locally. Using '{}' from local cache.",
                    recommended_model_id, selection.model.id
                );
                println!(
                    "Suggestion: run `tokensmith pull {}` for the optimized choice.",
                    recommended_model_id
                );
            }
            (selection, model_path)
        }
        Err(recommend_err) => {
            let (selection, model_path) =
                resolve_local_only_selection(paths, registry, &profile, task, mode, config)?;
            println!(
                "Warning: no optimal model fit for task '{}' mode '{}': {}",
                task,
                mode.as_str(),
                recommend_err
            );
            println!("Using local fallback model '{}'.", selection.model.id);
            println!(
                "Suggestion: pull a smaller code model or free memory, then retry `tokensmith recommend --task {} --mode {}`.",
                task,
                mode.as_str()
            );
            (selection, model_path)
        }
    };

    if let Some(ctx_override) = ctx {
        let auto_ctx = selection.context_tokens;
        selection.context_tokens = ctx_override;
        println!(
            "Context override enabled: using --ctx {} (auto-selected was {}).",
            ctx_override, auto_ctx
        );
        if ctx_override > auto_ctx {
            println!(
                "Warning: forced context may exceed safe memory/runtime limits and can cause 400/503 errors."
            );
        }
    }

    if detach {
        let exe = std::env::current_exe()?;
        let log_path = paths.new_server_log_path();
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let mut cmd = std::process::Command::new(exe);
        cmd.arg("serve")
            .arg("--task")
            .arg(task.to_string())
            .arg("--mode")
            .arg(mode.as_str())
            .arg("--host")
            .arg(host.clone())
            .arg("--port")
            .arg(port.to_string())
            .arg("--model-id")
            .arg(selection.model.id.clone())
            .arg("--model-path")
            .arg(model_path.display().to_string());

        if let Some(url) = runtime::llama_cpp::spawn_llama_server(paths, &selection, &host, port)? {
            cmd.arg("--runtime-url").arg(url);
        }

        let child = cmd
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .spawn()
            .context("failed to spawn detached server")?;

        let state = ServerState::active(
            child.id(),
            &host,
            port,
            &task.to_string(),
            mode.as_str(),
            &selection.model.id,
            &model_path.display().to_string(),
            "tokensmith-axum",
            &log_path.display().to_string(),
        );
        state.save(paths)?;
        println!(
            "tokensmith server started (pid {}) on http://{}:{}",
            child.id(),
            host,
            port
        );
    } else {
        let runtime_url = runtime::llama_cpp::spawn_llama_server(paths, &selection, &host, port)?;
        let log_path = paths.new_server_log_path();
        let state = ServerState::active(
            std::process::id(),
            &host,
            port,
            &task.to_string(),
            mode.as_str(),
            &selection.model.id,
            &model_path.display().to_string(),
            "tokensmith-axum",
            &log_path.display().to_string(),
        );
        state.save(paths)?;

        server::run_server(
            task,
            mode,
            host,
            port,
            selection.model.id,
            model_path.display().to_string(),
            runtime_url,
        )
        .await?;
    }
    Ok(())
}

fn resolve_selection_with_local_fallback(
    paths: &TokensmithPaths,
    registry: &Registry,
    profile: &profiler::types::HardwareProfile,
    task: cli::Task,
    mode: Mode,
    config: &AppConfig,
    preferred: selector::Selection,
) -> Result<(selector::Selection, std::path::PathBuf)> {
    let preferred_model_dir = paths.model_dir(&preferred.model.id);
    std::fs::create_dir_all(&preferred_model_dir)?;
    let preferred_model_path = preferred_model_dir.join(format!("{}.gguf", preferred.model.id));
    if preferred_model_path.exists() {
        return Ok((preferred, preferred_model_path));
    }

    let local_models = local_models_in_registry(paths, registry);
    if local_models.is_empty() {
        return Err(anyhow!(
            "model file not found at {}. No local model cache found. Run `tokensmith pull {}` first.",
            preferred_model_path.display(),
            preferred.model.id
        ));
    }

    // Try best local fit for requested task first.
    let local_task_models: Vec<_> = local_models
        .iter()
        .filter(|m| m.task == task.to_string())
        .cloned()
        .collect();
    if !local_task_models.is_empty() {
        let local_registry = Registry {
            models: local_task_models,
        };
        if let Ok(sel) = recommend(&local_registry, profile, task, mode, &config.optimizer) {
            let path = paths
                .model_dir(&sel.model.id)
                .join(format!("{}.gguf", sel.model.id));
            return Ok((sel, path));
        }
    }

    // Last resort: use any local model, even if task mismatch, to keep UX moving.
    let mut candidates = local_models;
    candidates.sort_by_key(|m| size_rank(&m.size_class));
    candidates.reverse();
    for m in candidates {
        if let Some((quant, ctx, mut reasons)) =
            selector::heuristics::choose_quant_and_ctx(&m, mode, profile, &config.optimizer)
        {
            let threads = profile
                .performance_cores
                .unwrap_or_else(|| (profile.logical_cores.saturating_sub(2)).clamp(1, 16));
            reasons.push("fallback: using locally available model with task mismatch".to_string());
            let sel = selector::Selection {
                model: m.clone(),
                quantization: quant,
                context_tokens: ctx,
                threads,
                reasons,
            };
            let path = paths
                .model_dir(&sel.model.id)
                .join(format!("{}.gguf", sel.model.id));
            return Ok((sel, path));
        }
    }

    Err(anyhow!(
        "recommended model '{}' is not local, and no local model fits memory for this mode. Try `tokensmith pull {}` or lower memory pressure (close apps / reduce context).",
        preferred.model.id,
        preferred.model.id
    ))
}

fn resolve_local_only_selection(
    paths: &TokensmithPaths,
    registry: &Registry,
    profile: &profiler::types::HardwareProfile,
    task: cli::Task,
    mode: Mode,
    config: &AppConfig,
) -> Result<(selector::Selection, std::path::PathBuf)> {
    let local_models = local_models_in_registry(paths, registry);
    if local_models.is_empty() {
        return Err(anyhow!(
            "no local model cache found. Pull one with `tokensmith pull <model_id>` first."
        ));
    }

    let local_task_models: Vec<_> = local_models
        .iter()
        .filter(|m| m.task == task.to_string())
        .cloned()
        .collect();
    if !local_task_models.is_empty() {
        let local_registry = Registry {
            models: local_task_models,
        };
        if let Ok(sel) = recommend(&local_registry, profile, task, mode, &config.optimizer) {
            let path = paths
                .model_dir(&sel.model.id)
                .join(format!("{}.gguf", sel.model.id));
            return Ok((sel, path));
        }
    }

    let mut candidates = local_models;
    candidates.sort_by_key(|m| size_rank(&m.size_class));
    candidates.reverse();
    for m in candidates {
        if let Some((quant, ctx, mut reasons)) =
            selector::heuristics::choose_quant_and_ctx(&m, mode, profile, &config.optimizer)
        {
            let threads = profile
                .performance_cores
                .unwrap_or_else(|| (profile.logical_cores.saturating_sub(2)).clamp(1, 16));
            reasons.push("fallback: using locally available model with task mismatch".to_string());
            let sel = selector::Selection {
                model: m.clone(),
                quantization: quant,
                context_tokens: ctx,
                threads,
                reasons,
            };
            let path = paths
                .model_dir(&sel.model.id)
                .join(format!("{}.gguf", sel.model.id));
            return Ok((sel, path));
        }
    }

    Err(anyhow!(
        "no local model fits memory for mode '{}'. Pull a smaller model (for example qwen2.5-1.5b-instruct) or lower memory pressure.",
        mode.as_str()
    ))
}

fn local_models_in_registry(
    paths: &TokensmithPaths,
    registry: &Registry,
) -> Vec<model_registry::ModelEntry> {
    registry
        .models
        .iter()
        .filter(|m| {
            paths
                .model_dir(&m.id)
                .join(format!("{}.gguf", m.id))
                .exists()
        })
        .cloned()
        .collect()
}

fn size_rank(size_class: &str) -> u32 {
    let cleaned = size_class.trim_end_matches('b');
    if let Some((whole, frac)) = cleaned.split_once('.') {
        let w = whole.parse::<u32>().unwrap_or(0);
        let f = frac.parse::<u32>().unwrap_or(0);
        return w.saturating_mul(100).saturating_add(f);
    }
    cleaned.parse::<u32>().unwrap_or(0).saturating_mul(100)
}

fn cmd_status(paths: &TokensmithPaths) -> Result<()> {
    let state = ServerState::load(paths)?;
    if !state.active {
        println!("No active server.");
        return Ok(());
    }

    if let Some(pid) = state.pid {
        if !is_pid_alive(pid) {
            println!("State says active, but pid {} is not alive.", pid);
            return Ok(());
        }

        let sampler = monitor::sample::default_sampler();
        let snapshot = sampler.sample(pid, state.started_at).ok();
        print_status_snapshot(&state, snapshot.as_ref());
    } else {
        println!("State is active but PID is missing.");
    }

    Ok(())
}

async fn cmd_monitor(
    paths: &TokensmithPaths,
    interval: Duration,
    watch: bool,
    json: bool,
    warn_mem: Option<f32>,
    warn_cpu: Option<f32>,
) -> Result<()> {
    let state = ServerState::load(paths)?;
    if !state.active {
        println!("No active server.");
        return Ok(());
    }
    let pid = state
        .pid
        .ok_or_else(|| anyhow!("active server has no PID"))?;
    let sampler = monitor::sample::default_sampler();

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    while running.load(Ordering::SeqCst) {
        let snapshot = sampler.sample(pid, state.started_at)?;

        if json {
            println!("{}", serde_json::to_string(&snapshot)?);
        } else {
            print_monitor_frame(&state, &snapshot, watch, warn_mem, warn_cpu);
        }

        if json {
            if let Some(mem_pct) = warn_mem {
                if let Some(total) = snapshot.total_mem_bytes {
                    let current_pct = (snapshot.rss_bytes as f32 / total as f32) * 100.0;
                    if current_pct > mem_pct {
                        eprintln!(
                            "WARNING: memory {:.1}% exceeds threshold {:.1}%. Try `tokensmith throttle --mode fast` or `tokensmith stop`.",
                            current_pct, mem_pct
                        );
                    }
                }
            }
            if let Some(cpu_pct) = warn_cpu {
                if snapshot.cpu_percent > cpu_pct {
                    eprintln!(
                        "WARNING: CPU {:.1}% exceeds threshold {:.1}%. Try `tokensmith throttle --mode fast` or `tokensmith stop`.",
                        snapshot.cpu_percent, cpu_pct
                    );
                }
            }
        }
        tokio::time::sleep(interval).await;
    }

    Ok(())
}

async fn cmd_stop(paths: &TokensmithPaths, force_after: Duration) -> Result<()> {
    let mut state = ServerState::load(paths)?;
    if !state.active {
        println!("No active server.");
        return Ok(());
    }
    let pid = state.pid.ok_or_else(|| anyhow!("state missing PID"))?;

    terminate_pid(pid, force_after)?;

    state.active = false;
    state.pid = None;
    state.save(paths)?;
    println!("Stopped server process {}", pid);
    Ok(())
}

async fn cmd_throttle(
    paths: &TokensmithPaths,
    config: &AppConfig,
    registry: &Registry,
    mode: Mode,
) -> Result<()> {
    let state = ServerState::load(paths)?;
    if !state.active {
        return Err(anyhow!("no active server to throttle"));
    }

    let task = state.task.parse::<cli::Task>().unwrap_or(cli::Task::Chat);
    let host = state.host.clone();
    let port = state.port;

    cmd_stop(paths, Duration::from_secs(5)).await?;
    cmd_up(paths, config, registry, task, mode, None, host, port, true).await?;
    println!("Server throttled to {} mode", mode.as_str());
    Ok(())
}

fn cmd_ps(paths: &TokensmithPaths) -> Result<()> {
    let state = ServerState::load(paths)?;
    if !state.active {
        println!("No managed process is active.");
        return Ok(());
    }

    let alive = state.pid.map(is_pid_alive).unwrap_or(false);
    println!("PID\tACTIVE\tALIVE\tHOST\tPORT\tMODEL\tMODE");
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        state.pid.unwrap_or_default(),
        state.active,
        alive,
        state.host,
        state.port,
        state.model_id,
        state.mode
    );
    Ok(())
}

async fn cmd_logs(paths: &TokensmithPaths, follow: bool, calls: bool) -> Result<()> {
    let state = ServerState::load(paths)?;
    if state.log_path.is_empty() {
        return Err(anyhow!("no log file path in state"));
    }

    if follow {
        let mut cmd = if calls {
            let escaped_path = state.log_path.replace('\'', "'\"'\"'");
            let mut c = tokio::process::Command::new("sh");
            c.arg("-c").arg(format!(
                "tail -f '{}' | awk '/model_call/ {{ print; fflush(); }}'",
                escaped_path
            ));
            c
        } else {
            let mut c = tokio::process::Command::new("tail");
            c.arg("-f").arg(&state.log_path);
            c
        };
        let status = cmd.status().await?;
        if !status.success() {
            return Err(anyhow!("tail failed with status {status}"));
        }
    } else {
        let content = std::fs::read_to_string(&state.log_path)
            .with_context(|| format!("failed to read {}", state.log_path))?;
        if calls {
            for line in content.lines().filter(|l| l.contains("model_call")) {
                println!("{line}");
            }
        } else {
            print!("{}", content);
        }
    }

    Ok(())
}

fn cmd_install(paths: &TokensmithPaths, target: InstallTarget) -> Result<()> {
    match target {
        InstallTarget::LlamaCpp => install_llama_cpp(paths)?,
    }
    Ok(())
}

fn install_llama_cpp(paths: &TokensmithPaths) -> Result<()> {
    if runtime::llama_cpp::find_llama_server(paths).is_some() {
        println!("llama-server is already installed and discoverable.");
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        println!("Installing llama.cpp via Homebrew...");
        let status = std::process::Command::new("brew")
            .args(["install", "llama.cpp"])
            .status()
            .context("failed to run brew install llama.cpp")?;
        if !status.success() {
            return Err(anyhow!(
                "brew install llama.cpp failed with status {}",
                status
            ));
        }

        if let Some(path) = runtime::llama_cpp::find_llama_server(paths) {
            println!("Installed llama-server at {}", path.display());
            return Ok(());
        }
        Err(anyhow!(
            "install completed but llama-server is still not discoverable in PATH"
        ))
    }

    #[cfg(target_os = "linux")]
    {
        println!("Automatic install is not implemented for Linux yet.");
        println!("Install llama.cpp and ensure `llama-server` is on PATH.");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        println!("Automatic install is not implemented for Windows yet.");
        println!("Install llama.cpp and ensure `llama-server.exe` is on PATH.");
        Ok(())
    }
}
