use std::net::SocketAddr;
use std::sync::{atomic::AtomicU64, Arc};

use anyhow::Result;
use axum::{routing::post, Router};

use crate::{cli::Task, optimizer::profiles::Mode};

pub mod metrics;
pub mod openai;
pub mod sse;

#[derive(Clone)]
pub struct AppState {
    pub task: Task,
    pub mode: Mode,
    pub model_id: String,
    pub model_path: String,
    pub runtime_url: Option<String>,
    pub requests_served: Arc<AtomicU64>,
}

pub async fn run_server(
    task: Task,
    mode: Mode,
    host: String,
    port: u16,
    model_id: String,
    model_path: String,
    runtime_url: Option<String>,
) -> Result<()> {
    let state = AppState {
        task,
        mode,
        model_id,
        model_path,
        runtime_url,
        requests_served: Arc::new(AtomicU64::new(0)),
    };

    let app = Router::new()
        .route("/v1/chat/completions", post(openai::chat_completions))
        .route("/v1/completions", post(openai::completions))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
