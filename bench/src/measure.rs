//! One streamed chat request, measured.
//!
//! Streaming is not optional here: TTFT is the headline number, and you can
//! only see it by timing the first content chunk off the wire. `stream_options.
//! include_usage` makes the final chunk carry `usage`, so one request yields
//! latency *and* cache stats.

use futures_util::StreamExt;
use serde_json::{Value, json};
use std::time::Instant;

const CHAT_URL: &str = "https://api.x.ai/v1/chat/completions";

/// What one request cost us.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Time to first *content* token. The role-only opening chunk doesn't count.
    pub ttft_ms: u128,
    pub total_ms: u128,
    pub prompt_tokens: u64,
    pub cached_tokens: u64,
    pub completion_tokens: u64,
    /// First 60 chars of the reply, so you can eyeball that it answered.
    pub preview: String,
}

impl Sample {
    /// The money number: did this request ride a warm prefix?
    pub fn hit(&self) -> bool {
        self.cached_tokens > 0
    }

    /// Share of the prompt that was served from cache.
    pub fn cache_ratio(&self) -> f64 {
        if self.prompt_tokens == 0 {
            return 0.0;
        }
        self.cached_tokens as f64 / self.prompt_tokens as f64
    }
}

pub struct Request<'a> {
    pub model: &'a str,
    /// Prepended as a system message at index 0. Grok caches from the start of
    /// the messages array, so this block is the thing being cached.
    pub prefix: Option<&'a str>,
    pub user: &'a str,
    /// Routes to a specific backend. Keyed by topic (not user) on the cached
    /// path — that's what makes hits reliable across "different" users.
    pub conv_id: &'a str,
    pub max_tokens: u32,
}

pub async fn run(
    client: &reqwest::Client,
    api_key: &str,
    req: Request<'_>,
) -> Result<Sample, String> {
    let mut messages = Vec::with_capacity(2);
    if let Some(prefix) = req.prefix {
        messages.push(json!({ "role": "system", "content": prefix }));
    }
    messages.push(json!({ "role": "user", "content": req.user }));

    let body = json!({
        "model": req.model,
        "stream": true,
        "stream_options": { "include_usage": true },
        "max_tokens": req.max_tokens,
        "messages": messages,
    });

    let mut builder = client
        .post(target.url())
        .header("content-type", "application/json")
        .header("x-grok-conv-id", req.conv_id)
        .json(&body);
    if let Target::Direct { api_key } = target {
        builder = builder.bearer_auth(api_key);
    }

    let started = Instant::now();
    let resp = builder
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("{status}: {}", body.trim()));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut ttft_ms: Option<u128> = None;
    let mut text = String::new();
    let mut usage: Option<Value> = None;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("stream broke: {e}"))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        // SSE frames are newline-delimited; hold back the trailing partial line.
        while let Some(newline) = buf.find('\n') {
            let line = buf[..newline].trim().to_string();
            buf.drain(..=newline);

            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            if payload.is_empty() || payload == "[DONE]" {
                continue;
            }

            let Ok(event): Result<Value, _> = serde_json::from_str(payload) else {
                continue;
            };

            if let Some(delta) = event["choices"][0]["delta"]["content"].as_str()
                && !delta.is_empty()
            {
                ttft_ms.get_or_insert_with(|| started.elapsed().as_millis());
                text.push_str(delta);
            }

            // The usage-bearing chunk arrives last and has an empty `choices`.
            if event["usage"].is_object() {
                usage = Some(event["usage"].clone());
            }
        }
    }

    let total_ms = started.elapsed().as_millis();
    let usage = usage.ok_or("no usage chunk — is stream_options.include_usage set?")?;
    let details = &usage["prompt_tokens_details"];

    Ok(Sample {
        ttft_ms: ttft_ms.unwrap_or(total_ms),
        total_ms,
        prompt_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0),
        cached_tokens: details["cached_tokens"].as_u64().unwrap_or(0),
        completion_tokens: usage["completion_tokens"].as_u64().unwrap_or(0),
        preview: text.chars().take(60).collect::<String>().replace('\n', " "),
    })
}
