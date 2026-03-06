use std::{fmt::Display, str::FromStr, time::Duration};

use clap::{Parser, Subcommand, ValueEnum};

use crate::optimizer::profiles::Mode;

#[derive(Parser, Debug)]
#[command(
    name = "tokensmith",
    version,
    about = "Local LLM orchestrator and serving layer"
)]
#[command(
    after_help = "Examples:\n  tokensmith doctor\n  tokensmith recommend --task code --mode balanced\n  tokensmith up --task chat --mode fast --detach"
)]
pub struct Cli {
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Doctor,
    Recommend {
        #[arg(long)]
        task: Task,
        #[arg(long, default_value = "balanced")]
        mode: Mode,
    },
    Pull {
        model_id: String,
    },
    Up {
        #[arg(long)]
        task: Task,
        #[arg(long, default_value = "balanced")]
        mode: Mode,
        #[arg(long)]
        ctx: Option<u32>,
        #[arg(long, default_value_t = 8000)]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long)]
        detach: bool,
    },
    Status,
    Monitor {
        #[arg(long, default_value = "1s", value_parser = parse_duration)]
        interval: Duration,
        #[arg(long)]
        watch: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, value_parser = parse_percent)]
        warn_mem: Option<f32>,
        #[arg(long, value_parser = parse_percent)]
        warn_cpu: Option<f32>,
    },
    Stop {
        #[arg(long, default_value = "5s", value_parser = parse_duration)]
        force_after: Duration,
    },
    Kill,
    Throttle {
        #[arg(long)]
        mode: Mode,
    },
    Ps,
    Logs {
        #[arg(long)]
        follow: bool,
        #[arg(long)]
        calls: bool,
    },
    Install {
        #[arg(value_enum)]
        target: InstallTarget,
    },
    #[command(hide = true)]
    Serve {
        #[arg(long)]
        task: Task,
        #[arg(long)]
        mode: Mode,
        #[arg(long)]
        host: String,
        #[arg(long)]
        port: u16,
        #[arg(long)]
        model_id: String,
        #[arg(long)]
        model_path: String,
        #[arg(long)]
        runtime_url: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum InstallTarget {
    #[value(name = "llama-cpp")]
    LlamaCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Task {
    Code,
    Chat,
}

impl Display for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Task::Code => write!(f, "code"),
            Task::Chat => write!(f, "chat"),
        }
    }
}

impl FromStr for Task {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "code" => Ok(Task::Code),
            "chat" => Ok(Task::Chat),
            _ => Err(anyhow::anyhow!("unsupported task: {s}")),
        }
    }
}

pub fn parse_duration(s: &str) -> Result<Duration, String> {
    if let Some(stripped) = s.strip_suffix("ms") {
        let ms = stripped.parse::<u64>().map_err(|e| e.to_string())?;
        return Ok(Duration::from_millis(ms));
    }
    if let Some(stripped) = s.strip_suffix('s') {
        let secs = stripped.parse::<u64>().map_err(|e| e.to_string())?;
        return Ok(Duration::from_secs(secs));
    }
    Err("duration must end with 's' or 'ms'".to_string())
}

pub fn parse_percent(s: &str) -> Result<f32, String> {
    let v = s
        .trim_end_matches('%')
        .parse::<f32>()
        .map_err(|e| e.to_string())?;
    Ok(v)
}
