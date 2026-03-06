use std::{
    net::TcpStream,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Result};

use crate::{config::paths::TokensmithPaths, selector::Selection};

pub fn find_llama_server(paths: &TokensmithPaths) -> Option<PathBuf> {
    let local = paths.bin_dir().join("llama-server");
    if local.exists() {
        return Some(local);
    }

    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|p| p.join("llama-server"))
            .find(|p| p.exists())
    })
}

pub fn spawn_llama_server(
    paths: &TokensmithPaths,
    selection: &Selection,
    host: &str,
    port: u16,
) -> Result<Option<String>> {
    let Some(binary) = find_llama_server(paths) else {
        return Ok(None);
    };

    // Use a neighboring port for llama.cpp runtime; tokensmith API listens on requested port.
    let runtime_port = port.saturating_add(1);
    let model_path = paths
        .model_dir(&selection.model.id)
        .join(format!("{}.gguf", selection.model.id));
    let log_path = paths.logs_dir().join(format!(
        "llama-server-{}.log",
        crate::utils::time::unix_timestamp()
    ));
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let mut cmd = Command::new(binary);
    cmd.arg("-m")
        .arg(model_path)
        .arg("--host")
        .arg(host)
        .arg("--port")
        .arg(runtime_port.to_string())
        .arg("-c")
        .arg(selection.context_tokens.to_string())
        .arg("-t")
        .arg(selection.threads.to_string())
        .stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file));

    let mut child = cmd.spawn()?;
    let runtime_addr = format!("{host}:{runtime_port}");
    let deadline = Instant::now() + Duration::from_secs(20);

    while Instant::now() < deadline {
        if TcpStream::connect(&runtime_addr).is_ok() {
            return Ok(Some(format!("http://{}", runtime_addr)));
        }
        if let Some(status) = child.try_wait()? {
            return Err(anyhow!(
                "llama-server exited early with status {}. Check log: {}",
                status,
                log_path.display()
            ));
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    Err(anyhow!(
        "llama-server did not become ready on {} within 20s. Check log: {}",
        runtime_addr,
        log_path.display()
    ))
}
