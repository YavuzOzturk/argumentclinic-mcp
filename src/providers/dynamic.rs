use std::env;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::RequestBuilder;
use serde_json::Value;

use crate::config::models::{AuthConfig, ConnectorConfig, KnownFormat};
use super::Provider;

// ── known-format templates ────────────────────────────────────────────────────

const OPENAI_TEMPLATE: &str =
    r#"{"model":"{{model}}","messages":[{"role":"system","content":"{{system}}"},{"role":"user","content":"{{prompt}}"}]}"#;

const ANTHROPIC_TEMPLATE: &str =
    r#"{"model":"{{model}}","max_tokens":2048,"system":"{{system}}","messages":[{"role":"user","content":"{{prompt}}"}]}"#;

const OLLAMA_TEMPLATE: &str =
    r#"{"model":"{{model}}","prompt":"{{system}}\n\n{{prompt}}","stream":false}"#;

const GEMINI_TEMPLATE: &str =
    r#"{"contents":[{"parts":[{"text":"{{system}}\n\n{{prompt}}"}]}]}"#;

const OPENAI_RESPONSE_PATH: &str = "choices[0].message.content";
const ANTHROPIC_RESPONSE_PATH: &str = "content[0].text";
const OLLAMA_RESPONSE_PATH: &str = "response";
const GEMINI_RESPONSE_PATH: &str = "candidates[0].content.parts[0].text";

// ── provider ──────────────────────────────────────────────────────────────────

pub struct DynamicProvider {
    connector: ConnectorConfig,
    model: String,
    http: reqwest::Client,
}

impl DynamicProvider {
    pub fn new(connector: ConnectorConfig, model: String) -> Self {
        Self {
            connector,
            model,
            http: reqwest::Client::new(),
        }
    }

    fn request_template(&self) -> Result<&str> {
        if let Some(ref fmt) = self.connector.format {
            return Ok(match fmt {
                KnownFormat::Openai => OPENAI_TEMPLATE,
                KnownFormat::Anthropic => ANTHROPIC_TEMPLATE,
                KnownFormat::Ollama => OLLAMA_TEMPLATE,
                KnownFormat::Gemini => GEMINI_TEMPLATE,
            });
        }
        self.connector
            .request_template
            .as_deref()
            .ok_or_else(|| anyhow!("connector has neither `format` nor `request_template`"))
    }

    fn effective_response_path(&self) -> Result<&str> {
        if let Some(ref path) = self.connector.response_path {
            return Ok(path.as_str());
        }
        match self.connector.format {
            Some(KnownFormat::Openai) => Ok(OPENAI_RESPONSE_PATH),
            Some(KnownFormat::Anthropic) => Ok(ANTHROPIC_RESPONSE_PATH),
            Some(KnownFormat::Ollama) => Ok(OLLAMA_RESPONSE_PATH),
            Some(KnownFormat::Gemini) => Ok(GEMINI_RESPONSE_PATH),
            None => Err(anyhow!("connector has neither `format` nor `response_path`")),
        }
    }

    fn apply_auth(
        &self,
        builder: RequestBuilder,
        system: &str,
        prompt: &str,
    ) -> Result<RequestBuilder> {
        match &self.connector.auth {
            AuthConfig::None => Ok(builder),
            AuthConfig::Bearer { token } => {
                let t = render(token, system, prompt, &self.model)?;
                Ok(builder.bearer_auth(t))
            }
            AuthConfig::ApiKey { header, key } => {
                let k = render(key, system, prompt, &self.model)?;
                // Gemini authenticates via query parameter, not a header
                if matches!(self.connector.format, Some(KnownFormat::Gemini)) {
                    return Ok(builder.query(&[("key", k)]));
                }
                let h = render(header, system, prompt, &self.model)?;
                Ok(builder.header(h, k))
            }
            AuthConfig::Basic { username, password } => {
                let u = render(username, system, prompt, &self.model)?;
                let p = render(password, system, prompt, &self.model)?;
                Ok(builder.basic_auth(u, Some(p)))
            }
        }
    }
}

#[async_trait]
impl Provider for DynamicProvider {
    fn name(&self) -> &str {
        "dynamic"
    }

    async fn complete(&self, system: &str, prompt: &str) -> Result<String> {
        // Build and validate request body — values are JSON-escaped before insertion
        let template = self.request_template()?;
        let body_str = render_body(template, system, prompt, &self.model)?;
        let body: Value = serde_json::from_str(&body_str)
            .with_context(|| format!("rendered request template is not valid JSON:\n{body_str}"))?;

        // URL supports {{model}}, {{env.VAR}} — raw substitution, no JSON-escaping
        let url = render(&self.connector.url, system, prompt, &self.model)?;

        let mut builder = self.http.post(&url).json(&body);

        // Format-specific extra headers
        if matches!(self.connector.format, Some(KnownFormat::Anthropic)) {
            builder = builder.header("anthropic-version", "2023-06-01");
        }

        // User-defined custom headers
        for (key, val) in &self.connector.headers {
            let rendered = render(val, system, prompt, &self.model)?;
            builder = builder.header(key.as_str(), rendered);
        }

        builder = self.apply_auth(builder, system, prompt)?;

        let response = builder
            .send()
            .await
            .with_context(|| format!("HTTP request to {url} failed"))?;

        let status = response.status();
        if !status.is_success() {
            let raw = response.text().await.unwrap_or_default();
            return Err(anyhow!("{url} responded with {status}: {raw}"));
        }

        let json: Value = response
            .json()
            .await
            .context("failed to parse response as JSON")?;

        let path = self.effective_response_path()?;
        extract_text(&json, path)
            .ok_or_else(|| anyhow!("response path '{path}' not found in response: {json}"))
    }
}

// ── template rendering ────────────────────────────────────────────────────────

/// Escape `s` for embedding inside a JSON string (strips the surrounding quotes
/// that `serde_json::to_string` adds).
fn json_escape(s: &str) -> String {
    let quoted = serde_json::to_string(s).expect("string serialization is infallible");
    quoted[1..quoted.len() - 1].to_string()
}

/// Render template variables for use in URLs, headers, and auth values.
/// Values are inserted as-is — no JSON-escaping.
fn render(template: &str, system: &str, prompt: &str, model: &str) -> Result<String> {
    render_impl(template, system, prompt, model, false)
}

/// Render template variables for use in a JSON request body.
/// `system`, `prompt`, and env var values are JSON-escaped before insertion
/// so the result is a valid JSON string.
fn render_body(template: &str, system: &str, prompt: &str, model: &str) -> Result<String> {
    render_impl(
        template,
        &json_escape(system),
        &json_escape(prompt),
        model,
        true, // escape env vars too
    )
}

fn render_impl(
    template: &str,
    system: &str,
    prompt: &str,
    model: &str,
    escape_env: bool,
) -> Result<String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        rest = &rest[open + 2..];

        let close = rest
            .find("}}")
            .ok_or_else(|| anyhow!("unclosed '{{{{' in template"))?;
        let var = &rest[..close];
        rest = &rest[close + 2..];

        match var {
            "system" => out.push_str(system),
            "prompt" => out.push_str(prompt),
            "model" => out.push_str(model),
            env_var if env_var.starts_with("env.") => {
                let name = &env_var["env.".len()..];
                let val = env::var(name)
                    .map_err(|_| anyhow!("environment variable {name:?} is not set"))?;
                let val = if escape_env { json_escape(&val) } else { val };
                out.push_str(&val);
            }
            other => return Err(anyhow!("unknown template variable: {{{{{other}}}}}")),
        }
    }

    out.push_str(rest);
    Ok(out)
}

// ── response path extraction ──────────────────────────────────────────────────

/// Walk `value` by a dot/bracket path such as `choices[0].message.content`.
///
/// Each `.`-separated segment may end with `[N]` to index into an array.
/// Returns `None` if any step in the path is missing.
fn extract_text(value: &Value, path: &str) -> Option<String> {
    let mut current = value;

    for segment in path.split('.') {
        current = if let Some(bracket) = segment.find('[') {
            let key = &segment[..bracket];
            let idx_str = segment[bracket + 1..].trim_end_matches(']');
            let idx: usize = idx_str.parse().ok()?;
            current.get(key)?.get(idx)?
        } else {
            current.get(segment)?
        };
    }

    Some(match current {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_basic_variables() {
        let out = render("{{system}} / {{prompt}} / {{model}}", "SYS", "USR", "m").unwrap();
        assert_eq!(out, "SYS / USR / m");
    }

    #[test]
    fn render_body_escapes_quotes() {
        let out = render_body(r#"{"x":"{{system}}"}"#, r#"a "b" c"#, "p", "m").unwrap();
        // After rendering, must be valid JSON and round-trip cleanly
        let v: Value = serde_json::from_str(&out).expect("should be valid JSON");
        assert_eq!(v["x"], r#"a "b" c"#);
    }

    #[test]
    fn render_unknown_variable_errors() {
        let err = render("{{unknown}}", "", "", "").unwrap_err();
        assert!(err.to_string().contains("unknown template variable"));
    }

    #[test]
    fn render_unclosed_brace_errors() {
        let err = render("{{system", "", "", "").unwrap_err();
        assert!(err.to_string().contains("unclosed"));
    }

    #[test]
    fn extract_text_nested() {
        let v = json!({ "choices": [{ "message": { "content": "hello" } }] });
        assert_eq!(extract_text(&v, "choices[0].message.content").unwrap(), "hello");
    }

    #[test]
    fn extract_text_top_level() {
        let v = json!({ "response": "world" });
        assert_eq!(extract_text(&v, "response").unwrap(), "world");
    }

    #[test]
    fn extract_text_deeply_nested() {
        let v = json!({
            "candidates": [{ "content": { "parts": [{ "text": "deep" }] } }]
        });
        assert_eq!(
            extract_text(&v, "candidates[0].content.parts[0].text").unwrap(),
            "deep"
        );
    }

    #[test]
    fn extract_text_missing_path_returns_none() {
        let v = json!({ "a": 1 });
        assert!(extract_text(&v, "a.b.c").is_none());
    }
}
