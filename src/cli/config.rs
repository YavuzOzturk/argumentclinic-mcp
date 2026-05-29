use anyhow::Result;

use crate::config::{
    self,
    models::{Mode, ModelAssignment, ProviderConfig},
};

// ── top-level args ────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(clap::Subcommand)]
pub enum ConfigCommands {
    /// Print the current configuration (API keys are redacted)
    Show,
    /// Add or update a provider's credentials
    #[command(name = "set-provider")]
    SetProvider(SetProviderArgs),
    /// Assign a provider and model to a pipeline role
    #[command(name = "set-role")]
    SetRole(SetRoleArgs),
    /// Save the ArgumentClinic Pro API key
    #[command(name = "set-ag-key")]
    SetAgKey(SetAgKeyArgs),
    /// Switch between base and pro mode
    #[command(name = "set-mode")]
    SetMode(SetModeArgs),
}

// ── subcommand args ───────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct SetProviderArgs {
    /// Provider name (e.g. openai, anthropic, ollama, my-local-llm)
    pub name: String,
    /// API key for the provider
    #[arg(long)]
    pub api_key: String,
    /// Base URL for OpenAI-compatible or local providers
    #[arg(long)]
    pub base_url: Option<String>,
}

#[derive(clap::Args)]
pub struct SetRoleArgs {
    /// Pipeline role to configure
    #[arg(value_enum)]
    pub role: RoleArg,
    /// Provider name (must match a configured provider)
    pub provider: String,
    /// Model name (e.g. gpt-4o-mini, claude-3-5-haiku-20241022)
    pub model: String,
}

#[derive(clap::Args)]
pub struct SetAgKeyArgs {
    /// The ArgumentClinic Pro API key
    pub key: String,
}

#[derive(clap::Args)]
pub struct SetModeArgs {
    /// Mode to activate
    #[arg(value_enum)]
    pub mode: ModeArg,
}

// ── value enums ───────────────────────────────────────────────────────────────

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum RoleArg {
    Attacker,
    Defender,
    Judge,
    Synthesizer,
}

impl std::fmt::Display for RoleArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RoleArg::Attacker   => "attacker",
            RoleArg::Defender   => "defender",
            RoleArg::Judge      => "judge",
            RoleArg::Synthesizer => "synthesizer",
        })
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ModeArg {
    Base,
    Pro,
}

impl std::fmt::Display for ModeArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ModeArg::Base => "base",
            ModeArg::Pro  => "pro",
        })
    }
}

// ── dispatch ──────────────────────────────────────────────────────────────────

pub async fn run(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommands::Show             => cmd_show(),
        ConfigCommands::SetProvider(a)   => cmd_set_provider(a),
        ConfigCommands::SetRole(a)       => cmd_set_role(a),
        ConfigCommands::SetAgKey(a)      => cmd_set_ag_key(a),
        ConfigCommands::SetMode(a)       => cmd_set_mode(a),
    }
}

// ── commands ──────────────────────────────────────────────────────────────────

fn cmd_show() -> Result<()> {
    let cfg = config::load()?;

    let is_empty = cfg.providers.is_empty()
        && cfg.ag_api_key.is_none()
        && cfg.roles.attacker.is_none()
        && cfg.roles.defender.is_none()
        && cfg.roles.judge.is_none()
        && cfg.roles.synthesizer.is_none();

    if is_empty {
        println!("No configuration found. Run 'argumentclinic config set-provider' to get started.");
        return Ok(());
    }

    println!("Mode: {}", match cfg.mode {
        Mode::Base => "base",
        Mode::Pro  => "pro",
    });

    if let Some(key) = &cfg.ag_api_key {
        println!("AG API Key: {}", redact(key));
    }

    if !cfg.providers.is_empty() {
        println!("\nProviders:");
        let mut names: Vec<&String> = cfg.providers.keys().collect();
        names.sort();
        for name in names {
            let p = &cfg.providers[name];
            println!("  {name}:");
            match &p.api_key {
                Some(k) => println!("    api_key:  {}", redact(k)),
                None    => println!("    api_key:  (not set)"),
            }
            if let Some(url) = &p.base_url {
                println!("    base_url: {url}");
            }
            if p.connector.is_some() {
                println!("    connector: (configured)");
            }
        }
    }

    println!("\nRoles:");
    print_role("attacker",    &cfg.roles.attacker);
    print_role("defender",    &cfg.roles.defender);
    print_role("judge",       &cfg.roles.judge);
    print_role("synthesizer", &cfg.roles.synthesizer);

    Ok(())
}

fn cmd_set_provider(args: SetProviderArgs) -> Result<()> {
    let mut cfg = config::load()?;
    cfg.providers.insert(
        args.name.clone(),
        ProviderConfig {
            api_key:   Some(args.api_key),
            base_url:  args.base_url,
            connector: None,
        },
    );
    config::save(&cfg)?;
    println!("Provider '{}' configured.", args.name);
    Ok(())
}

fn cmd_set_role(args: SetRoleArgs) -> Result<()> {
    let mut cfg = config::load()?;
    let assignment = ModelAssignment {
        provider: args.provider.clone(),
        model:    args.model.clone(),
    };
    let role_str = args.role.to_string();
    match args.role {
        RoleArg::Attacker    => cfg.roles.attacker    = Some(assignment),
        RoleArg::Defender    => cfg.roles.defender    = Some(assignment),
        RoleArg::Judge       => cfg.roles.judge       = Some(assignment),
        RoleArg::Synthesizer => cfg.roles.synthesizer = Some(assignment),
    }
    config::save(&cfg)?;
    println!("Role '{role_str}' set to {}/{}", args.provider, args.model);
    Ok(())
}

fn cmd_set_ag_key(args: SetAgKeyArgs) -> Result<()> {
    let mut cfg = config::load()?;
    cfg.ag_api_key = Some(args.key);
    config::save(&cfg)?;
    println!("ArgumentClinic API key saved.");
    Ok(())
}

fn cmd_set_mode(args: SetModeArgs) -> Result<()> {
    let mut cfg = config::load()?;
    let mode_str = args.mode.to_string();
    cfg.mode = match args.mode {
        ModeArg::Base => Mode::Base,
        ModeArg::Pro  => Mode::Pro,
    };
    config::save(&cfg)?;
    println!("Mode set to {mode_str}.");
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Show the first 4 characters of a key followed by `****`.
fn redact(key: &str) -> String {
    // Use char_indices so we slice on a char boundary, not a byte boundary.
    let prefix = key
        .char_indices()
        .nth(4)
        .map(|(i, _)| &key[..i])
        .unwrap_or(key);
    format!("{prefix}****")
}

fn print_role(label: &str, assignment: &Option<ModelAssignment>) {
    match assignment {
        Some(a) => println!("  {label}: {}/{}", a.provider, a.model),
        None    => println!("  {label}: (not set)"),
    }
}
