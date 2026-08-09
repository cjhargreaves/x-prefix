//! Holds `trend_count` prefix blocks in memory, refreshed from live X data.

use clients::x;
use context::Trend;
use std::collections::HashMap;
use std::sync::RwLock;

pub struct Allocator {
    bearer: String,
    trend_count: usize,
    trends: RwLock<HashMap<String, Trend>>,
}

impl Allocator {
    pub fn new(bearer: String, trend_count: usize) -> Self {
        Self {
            bearer,
            trend_count,
            trends: RwLock::new(HashMap::new()),
        }
    }

    pub async fn refresh(&self, client: &reqwest::Client) {
        let top_trends = x::trends(client, &self.bearer, x::UNITED_STATES).await;

        let mut built = HashMap::new();
        for trend in top_trends.into_iter().take(self.trend_count) {
            let posts = x::posts(client, &self.bearer, &trend.name, 8).await;
            if posts.is_empty() {
                continue;
            }
            built.insert(trend.name.clone(), context::build(&trend.name, &posts));
        }

        *self.trends.write().unwrap() = built;
    }

    pub fn lookup(&self, user_text: &str) -> Option<Trend> {
        let haystack = user_text.to_lowercase();
        self.trends
            .read()
            .unwrap()
            .values()
            .find(|trend| haystack.contains(&trend.key.to_lowercase()))
            .cloned()
    }

    pub fn active_count(&self) -> usize {
        self.trends.read().unwrap().len()
    }
}
