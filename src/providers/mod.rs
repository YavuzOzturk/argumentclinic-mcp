pub mod anthropic;
pub mod deepseek;
pub mod dynamic;
pub mod gemini;
pub mod generic;
pub mod grok;
pub mod openai;

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;

use crate::config::models::{ModelAssignment, ProviderConfig};

/// A single-turn text completion backed by an LLM provider.
///
/// Both `system` and `prompt` are required; providers that don't support a
/// dedicated system role should concatenate them in whatever way makes sense
/// for that API.
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    async fn complete(&self, system: &str, prompt: &str) -> Result<String>;
}

/// Construct the right [`Provider`] for `assignment` using credentials from `providers`.
///
/// If `ProviderConfig` has a `connector`, a [`dynamic::DynamicProvider`] is built
/// regardless of the provider name. Otherwise the name is matched against the
/// built-in set and `api_key` is required.
pub fn build_provider(
    assignment: &ModelAssignment,
    providers: &HashMap<String, ProviderConfig>,
) -> Result<Arc<dyn Provider>> {
    let config = providers.get(&assignment.provider).ok_or_else(|| {
        anyhow!(
            "provider '{}' is not configured; add it under the `providers` key in config.yaml",
            assignment.provider
        )
    })?;

    // A connector definition takes precedence — any HTTP endpoint, any format.
    if let Some(connector) = config.connector.clone() {
        return Ok(Arc::new(dynamic::DynamicProvider::new(
            connector,
            assignment.model.clone(),
        )));
    }

    // Built-in named providers all require an api_key.
    let api_key = config.api_key.clone().ok_or_else(|| {
        anyhow!(
            "provider '{}' has no `api_key` in config (and no `connector`)",
            assignment.provider
        )
    })?;
    let model = assignment.model.clone();

    let provider: Arc<dyn Provider> = match assignment.provider.as_str() {
        "openai" => Arc::new(openai::OpenAiProvider::new(api_key, model)),
        "anthropic" => Arc::new(anthropic::AnthropicProvider::new(api_key, model)),
        "deepseek" => Arc::new(deepseek::DeepSeekProvider::new(api_key, model)),
        "gemini" => Arc::new(gemini::GeminiProvider::new(api_key, model)),
        "grok" => Arc::new(grok::GrokProvider::new(api_key, model)),
        name => {
            let base_url = config.base_url.clone().ok_or_else(|| {
                anyhow!(
                    "provider '{name}' is not a built-in name and has no `base_url` configured"
                )
            })?;
            Arc::new(generic::GenericProvider::new(api_key, model, base_url))
        }
    };

    Ok(provider)
}
