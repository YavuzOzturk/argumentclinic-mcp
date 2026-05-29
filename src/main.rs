mod api;
mod cli;
mod config;
mod db;
mod mcp;
mod pipeline;
mod providers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    cli::run().await
}
