use crate::allocator::Allocator;
use std::env;
use std::sync::Arc;

const DEFAULT_TREND_COUNT: usize = 5;

#[derive(Clone)]
pub struct AppState {
    pub client: reqwest::Client,
    pub api_key: String,
    pub allocator: Arc<Allocator>,
}

impl AppState {
    pub fn from_env() -> Self {
        let trend_count = env::var("TREND_COUNT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_TREND_COUNT);

        let x_bearer = env::var("X_BEARER_TOKEN").expect("X_BEARER_TOKEN not set");

        Self {
            client: reqwest::Client::new(),
            api_key: env::var("XAI_API_KEY").expect("XAI_API_KEY not set"),
            allocator: Arc::new(Allocator::new(x_bearer, trend_count)),
        }
    }
}
