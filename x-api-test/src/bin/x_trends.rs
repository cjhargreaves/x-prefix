// Basic example: fetch current X trends using the bearer token (app-only auth).
//
// Run with: cargo run --bin x_trends
//
// Uses X API v2 trends-by-woeid. WOEID 1 = worldwide. Swap for a specific
// location's WOEID if you want regional trends instead.

use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let bearer = env::var("X_BEARER_TOKEN").expect("X_BEARER_TOKEN not set in .env");

    let woeid = 1; // worldwide
    let url = format!("https://api.twitter.com/2/trends/by/woeid/{woeid}");

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(&bearer)
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;

    println!("status: {status}");
    println!("{}", serde_json::to_string_pretty(&body)?);

    Ok(())
}
