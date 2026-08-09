use std::env;

#[derive(Clone)]
pub struct AppState {
    pub client: reqwest::Client,
    pub api_key: String,
}

impl AppState {
    pub fn from_env() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: env::var("XAI_API_KEY").expect("XAI_API_KEY not set"),
        }
    }
}
