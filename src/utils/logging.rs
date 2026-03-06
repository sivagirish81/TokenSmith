use std::{fs::OpenOptions, path::Path};

use anyhow::Result;
use tracing_subscriber::EnvFilter;

pub fn init_logging(verbosity: u8) -> Result<()> {
    let level = match verbosity {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
    Ok(())
}

pub fn init_background_logging(log_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(log_dir)?;
    let path = log_dir.join("tokensmith.log");
    let file = OpenOptions::new().create(true).append(true).open(path)?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file)
        .try_init();
    Ok(())
}
