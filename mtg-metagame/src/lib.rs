#![expect(dead_code)]
mod db;
mod models;
mod scraper;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "mtg-metagame", about = "MTGGoldfish metagame scraper")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scrape data from `MTGGoldfish`
    Scrape {
        /// Read pages from a local directory instead of fetching from the web.
        /// Files should be named by URL path, e.g. `tournaments_standard_page_1`
        /// for `/tournaments/standard?page=1`.
        #[arg(long)]
        local_dir: Option<PathBuf>,

        #[command(subcommand)]
        target: ScrapeTarget,
    },
    /// Show database statistics
    Stats {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },
}

#[derive(Subcommand)]
enum ScrapeTarget {
    /// Scrape tournament decklists via date range search
    Tournaments {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Start date (MM/DD/YYYY). Defaults to 14 days ago.
        #[arg(long)]
        from: Option<String>,

        /// End date (MM/DD/YYYY). Defaults to today.
        #[arg(long)]
        to: Option<String>,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },
    /// Scrape tournaments by their `MTGGoldfish` IDs (e.g. 62212 62177)
    Tournament {
        /// One or more numeric `MTGGoldfish` tournament IDs
        #[arg(required = true)]
        ids: Vec<i32>,

        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },
    /// Scrape metagame archetype index
    Metagame {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },
}

/// # Panics
///
/// Panics if the tokio runtime cannot be created.
pub fn run() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        if let Err(e) = run_command(cli.command).await {
            tracing::error!("Error: {e:#}");
            std::process::exit(1);
        }
    });
}

async fn run_command(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Scrape { local_dir, target } => {
            let fetcher = match local_dir {
                Some(dir) => scraper::Fetcher::local(&dir)?,
                None => scraper::Fetcher::http()?,
            };
            match target {
                ScrapeTarget::Tournaments { format, from, to, db } => {
                    let today = chrono::Utc::now().date_naive();
                    let from_date = match &from {
                        Some(s) => chrono::NaiveDate::parse_from_str(s, "%m/%d/%Y")?,
                        None => today - chrono::Days::new(14),
                    };
                    let to_date = match &to {
                        Some(s) => chrono::NaiveDate::parse_from_str(s, "%m/%d/%Y")?,
                        None if from.is_some() => from_date,
                        None => today,
                    };
                    let pool = db::connect(&db).await?;
                    db::migrate(&pool).await?;
                    scraper::tournament::scrape_tournaments(&pool, &fetcher, &format, from_date, to_date).await?;
                }
                ScrapeTarget::Tournament { ids, format, db } => {
                    let pool = db::connect(&db).await?;
                    db::migrate(&pool).await?;
                    for id in ids {
                        scraper::tournament::scrape_single_tournament(&pool, &fetcher, id, &format).await?;
                    }
                }
                ScrapeTarget::Metagame { format, db } => {
                    let pool = db::connect(&db).await?;
                    db::migrate(&pool).await?;
                    scraper::metagame::scrape_metagame(&pool, &fetcher, &format).await?;
                }
            }
        }
        Commands::Stats { format, db } => {
            let pool = db::connect(&db).await?;
            db::migrate(&pool).await?;
            db::stats(&pool, &format).await?;
        }
    }
    Ok(())
}
