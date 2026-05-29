#[derive(clap::Args)]
pub struct ServeArgs {}

pub async fn run(_args: ServeArgs) -> anyhow::Result<()> {
    crate::mcp::server::McpServer::new().run().await
}
