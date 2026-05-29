use super::Provider;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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

/// Any OpenAI-compatible endpoint. `base_url` should not include a trailing slash.
pub struct GenericProvider {
    api_key: String,
    model: String,
    base_url: String,
    http: reqwest::Client,
}

impl GenericProvider {
    pub fn new(api_key: String, model: String, base_url: String) -> Self {
        Self {
            api_key,
            model,
            base_url,
            http: reqwest::Client::new(),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url)
    }
}

#[async_trait]
impl Provider for GenericProvider {
    fn name(&self) -> &str {
        "generic"
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
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("request to generic chat completions endpoint failed")?;

        let status = response.status();
        if !status.is_success() {
            let raw = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiError>(&raw)
                .map(|e| e.error.message)
                .unwrap_or(raw);
            return Err(anyhow!("{} {status}: {message}", self.base_url));
        }

        let chat: ChatResponse = response
            .json()
            .await
            .context("failed to deserialize chat completions response")?;

        let choice = chat
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("{} returned an empty choices array", self.base_url))?;

        choice
            .message
            .content
            .ok_or_else(|| anyhow!("{} choice had no content (finish_reason: {:?})", self.base_url, choice.finish_reason))
    }
}
