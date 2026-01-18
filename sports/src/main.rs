use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "sports")]
#[command(about = "Sports data scraper and database importer")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Baseball Reference commands
    #[command(subcommand)]
    Baseball(baseballref::cli::BaseballCommands),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("sports=info".parse()?))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Baseball(baseball_cmd) => {
            baseballref::cli::handle_command(baseball_cmd).await?;
        }
    }

    Ok(())
}
