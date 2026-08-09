//! Fire one request at the running gateway.
//!
//! Start the gateway (`cargo run`), then in another shell:
//!   cargo run --bin run-test -- "what happened with nvidia"
//!   cargo run --bin run-test -- "catch me up" Sophie

use clients::grok::{self, ChatMessage};
use reqwest::header::{HeaderMap, HeaderValue};

const GATEWAY: &str = "http://127.0.0.1:3000/v1";

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let prompt = args.next().unwrap_or_else(|| "say ok".to_string());
    let trend = args.next();

    let mut client_builder = reqwest::Client::builder();
    if let Some(trend) = &trend {
        let mut headers = HeaderMap::new();
        headers.insert("x-trend", HeaderValue::from_str(trend).unwrap());
        client_builder = client_builder.default_headers(headers);
    }
    let client = client_builder.build().unwrap();

    let resp = grok::chat(
        &client,
        GATEWAY,
        "unused",
        "grok-4",
        &[ChatMessage::user(prompt)],
        64,
    )
    .await;

    println!("{}", resp.choices[0].message.content);
    println!(
        "prompt {} / cached {}",
        resp.usage.prompt_tokens, resp.usage.prompt_tokens_details.cached_tokens
    );
}
