//! Injects the named trend's prefix, forwards to xAI, streams the reply back.

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use serde_json::json;

use crate::state::AppState;

const UPSTREAM: &str = "https://api.x.ai/v1/chat/completions";
const CONV_HEADER: &str = "x-grok-conv-id";
const TREND_HEADER: &str = "x-trend";

pub async fn handle(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let trend = headers
        .get(TREND_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|key| state.allocator.get(key));

    let (body, conv_id) = match trend {
        Some(trend) => {
            println!(
                "inject trend={} prefix_bytes={} conv_id={}",
                trend.key,
                trend.prefix.len(),
                trend.conv_id
            );
            (inject(&body, &trend.prefix), Some(trend.conv_id.clone()))
        }
        None => (
            body,
            headers
                .get(CONV_HEADER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
        ),
    };

    let mut request = state
        .client
        .post(UPSTREAM)
        .bearer_auth(&state.api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body);

    if let Some(conv_id) = conv_id {
        request = request.header(CONV_HEADER, conv_id);
    }

    let upstream = request.send().await.unwrap();

    let status = upstream.status();

    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .cloned()
        .unwrap_or(header::HeaderValue::from_static("application/json"));

    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap()
}

pub async fn list_trends(State(state): State<AppState>) -> Json<Vec<String>> {
    Json(state.allocator.keys())
}

pub async fn get_trend(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<String, StatusCode> {
    state
        .allocator
        .get(&key)
        .map(|trend| trend.prefix.clone())
        .ok_or(StatusCode::NOT_FOUND)
}

fn inject(body: &Bytes, prefix: &str) -> Bytes {
    let mut request: serde_json::Value = serde_json::from_slice(body).unwrap();

    request
        .get_mut("messages")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .insert(0, json!({ "role": "system", "content": prefix }));

    Bytes::from(serde_json::to_vec(&request).unwrap())
}
