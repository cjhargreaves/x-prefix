//! Holds `trend_count` prefix blocks in memory, refreshed from live X data.

use crate::prefix::{self, Trend};
use clients::x;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct Allocator {
    bearer: String,
    trend_count: usize,
    posts_per_trend: usize,
    trends: RwLock<HashMap<String, Arc<Trend>>>,
}

impl Allocator {
    pub fn new(bearer: String, trend_count: usize, posts_per_trend: usize) -> Self {
        Self {
            bearer,
            trend_count,
            posts_per_trend,
            trends: RwLock::new(HashMap::new()),
        }
    }

    pub async fn refresh(&self, client: &reqwest::Client) {
        let top_trends = x::trends(client, &self.bearer, x::UNITED_STATES).await;

        let mut built = HashMap::new();
        for name in top_trends.into_iter().take(self.trend_count) {
            let posts = x::posts(client, &self.bearer, &name, self.posts_per_trend).await;
            if posts.is_empty() {
                continue;
            }
            built.insert(name.clone(), Arc::new(prefix::build(&name, &posts)));
        }

        *self.trends.write().unwrap() = built;
    }

    pub fn get(&self, key: &str) -> Option<Arc<Trend>> {
        self.trends.read().unwrap().get(key).cloned()
    }

    pub fn keys(&self) -> Vec<String> {
        self.trends.read().unwrap().keys().cloned().collect()
    }

    pub fn active_count(&self) -> usize {
        self.trends.read().unwrap().len()
    }
}
