//! Pull live trends, then the top posts for one of them.
//!
//! cargo run -p clients --example trends
//! cargo run -p clients --example trends -- "nvidia"

use clients::x;

#[tokio::main]
async fn main() {
    dotenvy::from_filename("gateway/.env").ok();

    let bearer = std::env::var("X_BEARER_TOKEN").unwrap();

    let client = reqwest::Client::new();

    let trends = x::trends(&client, &bearer, x::UNITED_STATES).await;

    println!("── trends (US) ──");
    for t in trends.iter().take(10) {
        println!("  {}", t.name);
    }

    let topic = std::env::args()
        .nth(1)
        .unwrap_or_else(|| trends[0].name.clone());

    println!("\n── posts for {topic} ──");

    let posts = x::posts(&client, &bearer, &topic, 8).await;
    for p in &posts {
        println!(
            "  {:>6} likes {:>6} rt  {}",
            p.likes,
            p.retweets,
            p.text.replace('\n', " ").chars().take(90).collect::<String>()
        );
    }

    let trend = context::build(&topic, &posts);

    println!("\n── prefix ({} bytes) ──\n{}", trend.prefix.len(), trend.prefix);
    println!("\nconv_id: {}", trend.conv_id());

    // Same posts in, twice — must come out byte-identical or the cache trick
    // is dead on arrival.
    let again = context::build(&topic, &posts);
    assert_eq!(trend.prefix, again.prefix, "prefix is not deterministic!");
    println!("determinism check: pass");
}
