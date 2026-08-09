//! Fire one request at the running gateway.
//!
//! Start the gateway (`cargo run`), then in another shell: `cargo run --bin run-test`
//! Optional prompt: `cargo run --bin run-test -- "what happened with nvidia"`

use serde_json::{Value, json};

const GATEWAY: &str = "http://127.0.0.1:3000/v1/chat/completions";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "say ok".to_string());

    let resp: Value = reqwest::Client::new()
        .post(GATEWAY)
        .json(&json!({
            "model": "grok-4",
            "messages": [{ "role": "user", "content": prompt }]
        }))
        .send()
        .await?
        .json()
        .await?;

    let usage = &resp["usage"];
    println!("{}", resp["choices"][0]["message"]["content"]);
    println!(
        "prompt {} / cached {}",
        usage["prompt_tokens"], usage["prompt_tokens_details"]["cached_tokens"]
    );

    Ok(())
}
