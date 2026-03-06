use axum::{
    body::Body,
    extract::State,
    http::header,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use super::{metrics, sse, AppState};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<Message>,
    pub stream: Option<bool>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CompletionRequest {
    pub model: Option<String>,
    pub prompt: String,
    pub stream: Option<bool>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
struct Choice {
    index: u32,
    message: Message,
    finish_reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
}

#[derive(Debug, Clone, Serialize)]
struct CompletionChoice {
    text: String,
    index: u32,
    finish_reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct CompletionResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<CompletionChoice>,
}

pub async fn chat_completions(
    State(state): State<AppState>,
    Json(req): Json<ChatCompletionRequest>,
) -> Response {
    metrics::increment_requests(&state);
    let runtime_req = ChatCompletionRequest {
        model: Some(state.model_id.clone()),
        ..req.clone()
    };
    let req_model = req.model.clone().unwrap_or_else(|| state.model_id.clone());
    let stream = req.stream.unwrap_or(false);
    info!(
        target: "tokensmith::model_call",
        endpoint = "/v1/chat/completions",
        model = %req_model,
        stream = stream,
        runtime = %state.runtime_url.as_deref().unwrap_or("none"),
        "model_call start"
    );

    if stream {
        match maybe_proxy_chat_stream(&state, &runtime_req).await {
            Ok(Some(resp)) => {
                info!(
                    target: "tokensmith::model_call",
                    endpoint = "/v1/chat/completions",
                    model = %req_model,
                    stream = true,
                    "model_call proxied_stream"
                );
                return resp;
            }
            Ok(None) => {}
            Err(e) => {
                error!(
                    target: "tokensmith::model_call",
                    endpoint = "/v1/chat/completions",
                    model = %req_model,
                    stream = true,
                    error = %e,
                    "model_call proxy_error"
                );
                return (StatusCode::BAD_GATEWAY, format!("proxy error: {e}")).into_response();
            }
        }
        warn!(
            target: "tokensmith::model_call",
            endpoint = "/v1/chat/completions",
            model = %req_model,
            stream = true,
            "model_call fallback_stream_echo"
        );

        let text = extract_last_user(&req.messages)
            .map(|t| {
                format!(
                    "tokensmith stream echo [{}:{}]: {t}",
                    state.task,
                    state.mode.as_str()
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "tokensmith stream echo [{}:{}]",
                    state.task,
                    state.mode.as_str()
                )
            });
        return sse::stream_text(text).into_response();
    }

    match maybe_proxy_chat(&state, &runtime_req).await {
        Ok(Some(resp)) => {
            info!(
                target: "tokensmith::model_call",
                endpoint = "/v1/chat/completions",
                model = %req_model,
                stream = false,
                "model_call proxied"
            );
            resp.into_response()
        }
        Ok(None) => {
            warn!(
                target: "tokensmith::model_call",
                endpoint = "/v1/chat/completions",
                model = %req_model,
                stream = false,
                "model_call fallback_local_response"
            );
            let content = extract_last_user(&req.messages)
                .map(|t| {
                    format!(
                        "tokensmith local response [{}:{} {}]: {t}",
                        state.task,
                        state.mode.as_str(),
                        state.model_path
                    )
                })
                .unwrap_or_else(|| {
                    format!(
                        "tokensmith local response [{}:{} {}]",
                        state.task,
                        state.mode.as_str(),
                        state.model_path
                    )
                });
            let out = ChatCompletionResponse {
                id: format!("chatcmpl-{}", crate::utils::time::unix_timestamp()),
                object: "chat.completion".to_string(),
                created: crate::utils::time::unix_timestamp(),
                model: req.model.unwrap_or_else(|| state.model_id.clone()),
                choices: vec![Choice {
                    index: 0,
                    message: Message {
                        role: "assistant".to_string(),
                        content,
                    },
                    finish_reason: "stop".to_string(),
                }],
            };
            Json(out).into_response()
        }
        Err(e) => {
            error!(
                target: "tokensmith::model_call",
                endpoint = "/v1/chat/completions",
                model = %req_model,
                stream = false,
                error = %e,
                "model_call proxy_error"
            );
            (StatusCode::BAD_GATEWAY, format!("proxy error: {e}")).into_response()
        }
    }
}

pub async fn completions(
    State(state): State<AppState>,
    Json(req): Json<CompletionRequest>,
) -> Response {
    metrics::increment_requests(&state);
    let runtime_req = CompletionRequest {
        model: Some(state.model_id.clone()),
        ..req.clone()
    };
    let req_model = req.model.clone().unwrap_or_else(|| state.model_id.clone());
    let stream = req.stream.unwrap_or(false);
    info!(
        target: "tokensmith::model_call",
        endpoint = "/v1/completions",
        model = %req_model,
        stream = stream,
        runtime = %state.runtime_url.as_deref().unwrap_or("none"),
        "model_call start"
    );

    if stream {
        match maybe_proxy_completion_stream(&state, &runtime_req).await {
            Ok(Some(resp)) => {
                info!(
                    target: "tokensmith::model_call",
                    endpoint = "/v1/completions",
                    model = %req_model,
                    stream = true,
                    "model_call proxied_stream"
                );
                return resp;
            }
            Ok(None) => {}
            Err(e) => {
                error!(
                    target: "tokensmith::model_call",
                    endpoint = "/v1/completions",
                    model = %req_model,
                    stream = true,
                    error = %e,
                    "model_call proxy_error"
                );
                return (StatusCode::BAD_GATEWAY, format!("proxy error: {e}")).into_response();
            }
        }
        warn!(
            target: "tokensmith::model_call",
            endpoint = "/v1/completions",
            model = %req_model,
            stream = true,
            "model_call fallback_stream_echo"
        );

        return sse::stream_text(format!(
            "tokensmith stream completion [{}:{}]: {}",
            state.task,
            state.mode.as_str(),
            req.prompt
        ))
        .into_response();
    }

    match maybe_proxy_completion(&state, &runtime_req).await {
        Ok(Some(resp)) => {
            info!(
                target: "tokensmith::model_call",
                endpoint = "/v1/completions",
                model = %req_model,
                stream = false,
                "model_call proxied"
            );
            resp.into_response()
        }
        Ok(None) => {
            warn!(
                target: "tokensmith::model_call",
                endpoint = "/v1/completions",
                model = %req_model,
                stream = false,
                "model_call fallback_local_response"
            );
            let out = CompletionResponse {
                id: format!("cmpl-{}", crate::utils::time::unix_timestamp()),
                object: "text_completion".to_string(),
                created: crate::utils::time::unix_timestamp(),
                model: req.model.unwrap_or_else(|| state.model_id.clone()),
                choices: vec![CompletionChoice {
                    text: format!(
                        "tokensmith completion [{}:{} {}]: {}",
                        state.task,
                        state.mode.as_str(),
                        state.model_path,
                        req.prompt
                    ),
                    index: 0,
                    finish_reason: "stop".to_string(),
                }],
            };
            Json(out).into_response()
        }
        Err(e) => {
            error!(
                target: "tokensmith::model_call",
                endpoint = "/v1/completions",
                model = %req_model,
                stream = false,
                error = %e,
                "model_call proxy_error"
            );
            (StatusCode::BAD_GATEWAY, format!("proxy error: {e}")).into_response()
        }
    }
}

fn extract_last_user(messages: &[Message]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
}

async fn maybe_proxy_chat(
    state: &AppState,
    req: &ChatCompletionRequest,
) -> anyhow::Result<Option<Json<serde_json::Value>>> {
    let Some(base) = &state.runtime_url else {
        return Ok(None);
    };

    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
    let resp = client.post(url).json(req).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "runtime returned {} for /v1/chat/completions: {}",
            status,
            truncate_err_body(&body)
        ));
    }
    let v = resp.json::<serde_json::Value>().await?;
    Ok(Some(Json(v)))
}

async fn maybe_proxy_completion(
    state: &AppState,
    req: &CompletionRequest,
) -> anyhow::Result<Option<Json<serde_json::Value>>> {
    let Some(base) = &state.runtime_url else {
        return Ok(None);
    };

    let client = reqwest::Client::new();
    let url = format!("{}/v1/completions", base.trim_end_matches('/'));
    let resp = client.post(url).json(req).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "runtime returned {} for /v1/completions: {}",
            status,
            truncate_err_body(&body)
        ));
    }
    let v = resp.json::<serde_json::Value>().await?;
    Ok(Some(Json(v)))
}

async fn maybe_proxy_chat_stream(
    state: &AppState,
    req: &ChatCompletionRequest,
) -> anyhow::Result<Option<Response>> {
    let Some(base) = &state.runtime_url else {
        return Ok(None);
    };

    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", base.trim_end_matches('/'));
    let resp = client.post(url).json(req).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "runtime returned {} for streamed /v1/chat/completions: {}",
            status,
            truncate_err_body(&body)
        ));
    }

    let stream = resp
        .bytes_stream()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(body)?;
    Ok(Some(response))
}

async fn maybe_proxy_completion_stream(
    state: &AppState,
    req: &CompletionRequest,
) -> anyhow::Result<Option<Response>> {
    let Some(base) = &state.runtime_url else {
        return Ok(None);
    };

    let client = reqwest::Client::new();
    let url = format!("{}/v1/completions", base.trim_end_matches('/'));
    let resp = client.post(url).json(req).send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "runtime returned {} for streamed /v1/completions: {}",
            status,
            truncate_err_body(&body)
        ));
    }

    let stream = resp
        .bytes_stream()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(body)?;
    Ok(Some(response))
}

fn truncate_err_body(s: &str) -> String {
    const MAX: usize = 240;
    let cleaned = s.replace('\n', " ").trim().to_string();
    if cleaned.len() <= MAX {
        cleaned
    } else {
        format!("{}...", &cleaned[..MAX])
    }
}
