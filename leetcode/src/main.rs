use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "leetcode")]
#[command(about = "Local LeetCode practice harness: fetch problems, solve them with cargo test")]
struct Cli {
    #[command(subcommand)]
    command: leetcode::harness::cli::Command,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("leetcode=info")))
        .init();

    let cli = Cli::parse();
    leetcode::harness::cli::handle_command(cli.command).await
}
