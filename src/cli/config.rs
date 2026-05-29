#[derive(clap::Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(clap::Subcommand)]
pub enum ConfigCommands {
    /// Print current configuration
    Show,
    /// Set a configuration key
    Set {
        key: String,
        value: String,
    },
    /// Switch between base and pro mode
    Mode {
        #[arg(value_enum)]
        mode: ModeArg,
    },
}

#[derive(clap::ValueEnum, Clone)]
pub enum ModeArg {
    Base,
    Pro,
}

pub async fn run(_args: ConfigArgs) -> anyhow::Result<()> {
    todo!()
}
