use crate::allocator::Allocator;
use std::env;
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_TREND_COUNT: usize = 5;
const DEFAULT_POSTS_PER_TREND: usize = 8;
const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 300;

#[derive(Clone)]
pub struct AppState {
    pub client: reqwest::Client,
    pub api_key: String,
    pub allocator: Arc<Allocator>,
    pub refresh_interval: Duration,
}

impl AppState {
    pub fn from_env() -> Self {
        let trend_count = env::var("TREND_COUNT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_TREND_COUNT);

        let posts_per_trend = env::var("POSTS_PER_TREND")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_POSTS_PER_TREND);

        let refresh_interval_secs = env::var("REFRESH_INTERVAL_SECS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_REFRESH_INTERVAL_SECS);

        let x_bearer = env::var("X_BEARER_TOKEN").expect("X_BEARER_TOKEN not set");

        Self {
            client: reqwest::Client::new(),
            api_key: env::var("XAI_API_KEY").expect("XAI_API_KEY not set"),
            allocator: Arc::new(Allocator::new(x_bearer, trend_count, posts_per_trend)),
            refresh_interval: Duration::from_secs(refresh_interval_secs),
        }
    }
}
