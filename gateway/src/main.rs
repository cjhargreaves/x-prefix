// Step 1: dumb passthrough proxy.
//
// Accepts OpenAI-format chat requests and forwards them to xAI unchanged,
// streaming the response straight back. No matching, no mutation yet.

use axum::{
    Router,
    body::{Body, Bytes},
    extract::State,
    http::{StatusCode, header},
    response::Response,
    routing::post,
};
use std::env;

const UPSTREAM: &str = "https://api.x.ai/v1/chat/completions";
const LISTEN_ADDR: &str = "127.0.0.1:3000";

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    api_key: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let api_key = env::var("XAI_API_KEY").expect("XAI_API_KEY not set in .env");

    let state = AppState {
        client: reqwest::Client::new(),
        api_key,
    };

    let app = Router::new()
        .route("/v1/chat/completions", post(proxy))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(LISTEN_ADDR).await.unwrap();
    println!("gateway listening on http://{LISTEN_ADDR}");
    axum::serve(listener, app).await.unwrap();
}

async fn proxy(State(state): State<AppState>, body: Bytes) -> Result<Response, StatusCode> {
    let upstream = state
        .client
        .post(UPSTREAM)
        .bearer_auth(&state.api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| {
            eprintln!("upstream request failed: {e}");
            StatusCode::BAD_GATEWAY
        })?;

    let status = upstream.status();

    // Preserve content-type so SSE stays SSE on the way back.
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .cloned()
        .unwrap_or_else(|| header::HeaderValue::from_static("application/json"));

    // Dumb pipe: stream bytes through without parsing them.
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from_stream(upstream.bytes_stream()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
