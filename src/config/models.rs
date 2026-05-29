use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub ag_api_key: Option<String>,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    pub roles: RoleConfig,
}

// ── provider config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Required for built-in named providers (openai, anthropic, …).
    /// Not needed when `connector` is set — auth lives inside `ConnectorConfig`.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Used by the `generic` built-in provider only.
    #[serde(default)]
    pub base_url: Option<String>,
    /// When present, a `DynamicProvider` is built instead of a named built-in.
    #[serde(default)]
    pub connector: Option<ConnectorConfig>,
}

// ── dynamic connector ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    /// Target endpoint URL. Supports `{{model}}` and `{{env.VAR}}` placeholders.
    pub url: String,
    /// Authentication scheme. Defaults to no auth.
    #[serde(default)]
    pub auth: AuthConfig,
    /// Use a well-known request/response shape instead of a custom template.
    pub format: Option<KnownFormat>,
    /// Raw JSON template string. Used when `format` is absent.
    /// Supports `{{system}}`, `{{prompt}}`, `{{model}}`, `{{env.VAR}}`.
    pub request_template: Option<String>,
    /// Dot/bracket path into the JSON response, e.g. `choices[0].message.content`.
    /// Inferred from `format` when absent.
    pub response_path: Option<String>,
    /// Additional HTTP headers. Values support template variables.
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfig {
    None,
    Bearer { token: String },
    ApiKey { header: String, key: String },
    Basic { username: String, password: String },
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig::None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnownFormat {
    Openai,
    Anthropic,
    Ollama,
    Gemini,
}

// ── mode ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Base,
    Pro,
}

// ── role config ───────────────────────────────────────────────────────────────

/// Per-role model assignments (BASE mode: user-configured).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RoleConfig {
    pub attacker: Option<ModelAssignment>,
    pub defender: Option<ModelAssignment>,
    pub judge: Option<ModelAssignment>,
    pub synthesizer: Option<ModelAssignment>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelAssignment {
    pub provider: String,
    pub model: String,
}
