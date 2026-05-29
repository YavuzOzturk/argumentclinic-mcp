use super::Provider;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const GEMINI_BASE_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models";

// ── request types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct GenerateRequest {
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

// ── response types ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GenerateResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: CandidateContent,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
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

pub struct GeminiProvider {
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http: reqwest::Client::new(),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/{}:generateContent", GEMINI_BASE_URL, self.model)
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn complete(&self, system: &str, prompt: &str) -> Result<String> {
        let body = GenerateRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: format!("{system}\n\n{prompt}"),
                }],
            }],
        };

        let response = self
            .http
            .post(self.endpoint())
            .query(&[("key", &self.api_key)])
            .json(&body)
            .send()
            .await
            .context("request to Gemini generateContent failed")?;

        let status = response.status();
        if !status.is_success() {
            let raw = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiError>(&raw)
                .map(|e| e.error.message)
                .unwrap_or(raw);
            return Err(anyhow!("Gemini {status}: {message}"));
        }

        let gen: GenerateResponse = response
            .json()
            .await
            .context("failed to deserialize Gemini response")?;

        gen.candidates
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Gemini returned no candidates"))?
            .content
            .parts
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Gemini candidate had no parts"))?
            .text
            .ok_or_else(|| anyhow!("Gemini part had no text"))
    }
}
