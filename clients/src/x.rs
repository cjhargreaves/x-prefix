//! X API: trends and posts.

use serde::Deserialize;

const TRENDS: &str = "https://api.twitter.com/2/trends/by/woeid";
const SEARCH: &str = "https://api.twitter.com/2/tweets/search/recent";

pub const WORLDWIDE: u32 = 1;
pub const UNITED_STATES: u32 = 23424977;

pub struct Trend {
    pub name: String,
}

pub struct Post {
    pub id: String,
    pub text: String,
    pub created_at: String,
    pub likes: u64,
    pub retweets: u64,
    pub impressions: u64,
}

pub async fn trends(client: &reqwest::Client, bearer: &str, woeid: u32) -> Vec<Trend> {
    let body: TrendsBody = client
        .get(format!("{TRENDS}/{woeid}"))
        .bearer_auth(bearer)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    body.data
        .into_iter()
        .map(|t| Trend { name: t.trend_name })
        .collect()
}

pub async fn posts(client: &reqwest::Client, bearer: &str, query: &str, limit: usize) -> Vec<Post> {
    let search = format!("{query} -is:retweet lang:en");

    let body: SearchBody = client
        .get(SEARCH)
        .bearer_auth(bearer)
        .query(&[
            ("query", search.as_str()),
            ("max_results", "100"),
            ("sort_order", "relevancy"),
            ("tweet.fields", "public_metrics,created_at"),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let mut posts: Vec<Post> = body
        .data
        .into_iter()
        .map(|p| Post {
            id: p.id,
            text: p.text,
            created_at: p.created_at,
            likes: p.public_metrics.like_count,
            retweets: p.public_metrics.retweet_count,
            impressions: p.public_metrics.impression_count,
        })
        .collect();

    posts.sort_by_key(|p| std::cmp::Reverse(p.likes + p.retweets));

    posts.truncate(limit);

    posts
}

#[derive(Deserialize)]
struct TrendsBody {
    #[serde(default)]
    data: Vec<TrendItem>,
}

#[derive(Deserialize)]
struct TrendItem {
    trend_name: String,
}

#[derive(Deserialize)]
struct SearchBody {
    #[serde(default)]
    data: Vec<PostItem>,
}

#[derive(Deserialize)]
struct PostItem {
    id: String,
    text: String,
    created_at: String,
    public_metrics: Metrics,
}

#[derive(Deserialize)]
struct Metrics {
    like_count: u64,
    retweet_count: u64,
    impression_count: u64,
}
