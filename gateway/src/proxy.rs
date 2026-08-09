//! Passthrough. Adds the xAI key, forwards everything else untouched.

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, header},
    response::Response,
};

use crate::state::AppState;

const UPSTREAM: &str = "https://api.x.ai/v1/chat/completions";
const CONV_HEADER: &str = "x-grok-conv-id";

pub async fn handle(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {

    let mut request = state
        .client
        .post(UPSTREAM)
        .bearer_auth(&state.api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body);

    if let Some(conv) = headers.get(CONV_HEADER) {
        request = request.header(CONV_HEADER, conv);
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
