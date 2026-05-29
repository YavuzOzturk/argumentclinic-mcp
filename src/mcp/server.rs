use anyhow::anyhow;
use rmcp::{
    Error as McpError, ServerHandler, ServiceExt,
    handler::server::router::tool::ToolRouter,
    handler::server::tool::Parameters,
    model::{Implementation, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    config,
    pipeline::{
        Pipeline,
        attacker::Attacker,
        defender::Defender,
        judge::Judge,
        synthesizer::Synthesizer,
    },
    providers::build_provider,
};

use std::future::Future;

// ── tool parameter schema ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    /// The text or claim to analyze
    pub content: String,
    /// Optional context describing what the content is or how to interpret it
    pub context: Option<String>,
}

// ── server ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct McpServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl McpServer {
    #[tool(description = "Run adversarial analysis on a claim or argument. Returns the full debate transcript including attack, defense, judgment, and final verdict.")]
    async fn analyze(
        &self,
        Parameters(params): Parameters<AnalyzeParams>,
    ) -> Result<String, McpError> {
        let input = match params.context.as_deref() {
            Some(ctx) if !ctx.is_empty() => format!("{ctx}\n\n{}", params.content),
            _ => params.content,
        };
        run_analysis(&input)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        self.serve((tokio::io::stdin(), tokio::io::stdout()))
            .await
            .map_err(|e| anyhow!("MCP server failed to initialize: {e}"))?
            .waiting()
            .await
            .map_err(|e| anyhow!("MCP server exited with error: {e}"))?;
        Ok(())
    }
}

// Default field name is `tool_router` so no need for `router = ...`.
#[tool_handler]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "argumentclinic".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            ..ServerInfo::default()
        }
    }
}

// ── pipeline helper ───────────────────────────────────────────────────────────

async fn run_analysis(input: &str) -> anyhow::Result<String> {
    let cfg = config::load()?;
    let roles = &cfg.roles;

    macro_rules! require_role {
        ($field:ident, $name:literal) => {
            roles.$field.as_ref().ok_or_else(|| {
                anyhow!(
                    "no {name} role configured — run: argumentclinic config set-role {name} <provider> <model>",
                    name = $name
                )
            })?
        };
    }

    let attacker = Attacker::new(
        build_provider(require_role!(attacker, "attacker"), &cfg.providers)?,
    );
    let defender = Defender::new(
        build_provider(require_role!(defender, "defender"), &cfg.providers)?,
    );
    let judge = Judge::new(
        build_provider(require_role!(judge, "judge"), &cfg.providers)?,
    );
    let synthesizer = Synthesizer::new(
        build_provider(require_role!(synthesizer, "synthesizer"), &cfg.providers)?,
    );

    let result = Pipeline::new(attacker, defender, judge, synthesizer)
        .run(input)
        .await?;

    Ok(result.format_output())
}
