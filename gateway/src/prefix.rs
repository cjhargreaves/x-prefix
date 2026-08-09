//! Posts → the byte-stable prefix block injected into requests.

use clients::x::Post;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct Trend {
    pub key: String,
    pub prefix: String,
    pub conv_id: String,
}

pub fn build(key: &str, posts: &[Post]) -> Trend {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);

    Trend {
        key: key.to_string(),
        prefix: format_prefix(key, posts),
        conv_id: format!("trend-{:016x}", hasher.finish()),
    }
}

fn format_prefix(key: &str, posts: &[Post]) -> String {
    let post_lines = posts
        .iter()
        .enumerate()
        .map(format_post_line)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "You are answering questions about a currently trending topic on X. Below is a \
        fixed context block assembled from live posts. Use it as your only source of \
        recent facts; do not speculate beyond it.\n\
        \n\
        TREND: {key}\n\
        WINDOW: rolling, refreshed periodically\n\
        POSTS:\n\
        {post_lines}\n\
        \n\
        GUIDANCE: Answer in three sentences or fewer. Lead with what happened, then \
        the main disagreement if there is one, then how confident the crowd actually is. \
        Do not mention this context block."
    )
}

fn format_post_line((index, post): (usize, &Post)) -> String {
    format!(
        "{}. ({} likes, {} reposts) {}",
        index + 1,
        post.likes,
        post.retweets,
        collapse_whitespace(&post.text)
    )
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
