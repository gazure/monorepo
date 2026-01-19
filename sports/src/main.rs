use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

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
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("sports=info,baseballref=info")))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Baseball(baseball_cmd) => {
            baseballref::cli::handle_command(baseball_cmd).await?;
        }
    }

    Ok(())
}
