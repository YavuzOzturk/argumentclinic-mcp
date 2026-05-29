use super::Provider;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const CHAT_COMPLETIONS_URL: &str = "https://api.x.ai/v1/chat/completions";

// ── request types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: &'a str,
}

// ── response types ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: AssistantMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct AssistantMessage {
    content: Option<String>,
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

pub struct GrokProvider {
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl GrokProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Provider for GrokProvider {
    fn name(&self) -> &str {
        "grok"
    }

    async fn complete(&self, system: &str, prompt: &str) -> Result<String> {
        let body = ChatRequest {
            model: &self.model,
            messages: vec![
                Message { role: "system", content: system },
                Message { role: "user", content: prompt },
            ],
        };

        let response = self
            .http
            .post(CHAT_COMPLETIONS_URL)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("request to Grok chat completions failed")?;

        let status = response.status();
        if !status.is_success() {
            let raw = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiError>(&raw)
                .map(|e| e.error.message)
                .unwrap_or(raw);
            return Err(anyhow!("Grok {status}: {message}"));
        }

        let chat: ChatResponse = response
            .json()
            .await
            .context("failed to deserialize Grok response")?;

        let choice = chat
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Grok returned an empty choices array"))?;

        choice
            .message
            .content
            .ok_or_else(|| anyhow!("Grok choice had no content (finish_reason: {:?})", choice.finish_reason))
    }
}
