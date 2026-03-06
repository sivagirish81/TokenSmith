use std::convert::Infallible;
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream;
use futures::StreamExt;

pub fn stream_text(text: String) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let chunks: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();

    let stream = stream::iter(chunks.into_iter().enumerate()).map(|(i, token)| {
        let payload = serde_json::json!({
            "id": format!("chatcmpl-stream-{i}"),
            "object": "chat.completion.chunk",
            "choices": [{
                "index": 0,
                "delta": { "content": format!("{} ", token) },
                "finish_reason": serde_json::Value::Null
            }]
        });
        Ok(Event::default().data(payload.to_string()))
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(5)))
}
