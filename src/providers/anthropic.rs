use super::Provider;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 2048;

// ── request types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    system: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

// ── response types ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

// ── error body ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ApiError {
    error: ApiErrorDetail,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
}

// ── provider ─────────────────────────────────────────────────────────────────

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn complete(&self, system: &str, prompt: &str) -> Result<String> {
        let body = MessagesRequest {
            model: &self.model,
            max_tokens: MAX_TOKENS,
            system,
            messages: vec![Message { role: "user", content: prompt }],
        };

        let response = self
            .http
            .post(MESSAGES_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await
            .context("request to Anthropic messages API failed")?;

        let status = response.status();
        if !status.is_success() {
            let raw = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiError>(&raw)
                .map(|e| e.error.message)
                .unwrap_or(raw);
            return Err(anyhow!("Anthropic {status}: {message}"));
        }

        let msg: MessagesResponse = response
            .json()
            .await
            .context("failed to deserialize Anthropic response")?;

        msg.content
            .into_iter()
            .find(|b| b.kind == "text")
            .and_then(|b| b.text)
            .ok_or_else(|| anyhow!("Anthropic response contained no text content block"))
    }
}
