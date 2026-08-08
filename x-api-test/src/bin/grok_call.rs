// Basic example: call the xAI Grok chat completions API and print the
// response including cache-hit info from usage.prompt_tokens_details.
//
// Run with: cargo run --bin grok_call

use serde_json::json;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let api_key = env::var("XAI_API_KEY").expect("XAI_API_KEY not set in .env");

    let body = json!({
        "model": "grok-4",
        "messages": [
            { "role": "system", "content": "You are a concise assistant." },
            { "role": "user", "content": "In one sentence, what is prefix caching?" }
        ]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.x.ai/v1/chat/completions")
        .bearer_auth(&api_key)
        // Keying this by topic (instead of per-user) is the whole trick later —
        // for this smoke test a static id is fine.
        .header("x-grok-conv-id", "test-conversation")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;

    println!("status: {status}");
    println!("{}", serde_json::to_string_pretty(&body)?);

    if let Some(cached) = body["usage"]["prompt_tokens_details"]["cached_tokens"].as_i64() {
        println!("\ncached_tokens: {cached}");
    }

    Ok(())
}
