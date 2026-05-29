pub mod analyze;
pub mod config;
pub mod serve;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "argumentclinic",
    version,
    about = "Adversarial reasoning CLI and MCP server"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run adversarial analysis on a claim
    Analyze(analyze::AnalyzeArgs),
    /// Start the MCP server (for Cursor and Claude Desktop)
    Serve(serve::ServeArgs),
    /// Manage configuration
    Config(config::ConfigArgs),
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Analyze(args) => analyze::run(args).await,
        Commands::Serve(args)  => serve::run(args).await,
        Commands::Config(args) => config::run(args).await,
    }
}
