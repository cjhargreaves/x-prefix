//! xAI API: chat completions.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
}

#[derive(Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Deserialize)]
pub struct Choice {
    pub message: ChatMessage,
}

#[derive(Deserialize)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub prompt_tokens_details: PromptTokensDetails,
}

#[derive(Deserialize)]
pub struct PromptTokensDetails {
    pub cached_tokens: u64,
}

pub async fn chat(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    messages: &[ChatMessage],
    max_tokens: u32,
) -> ChatResponse {
    let body = ChatRequestBody {
        model,
        messages,
        max_tokens,
        stream: false,
    };

    client
        .post(format!("{base_url}/chat/completions"))
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

#[derive(Serialize)]
struct ChatRequestBody<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    max_tokens: u32,
    stream: bool,
}
