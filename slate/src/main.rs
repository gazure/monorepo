//! slate CLI: sync schedules, inspect changes, and serve calendar feeds.

use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use slate::{config::Config, espn, ics, server, store::Store, sync_all};

#[derive(Parser)]
#[command(name = "slate", about = "calendar feeds for sports schedules")]
struct Cli {
    /// Path to the configuration file.
    #[arg(short, long, env = "SLATE_CONFIG", default_value = "slate.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Fetch every configured feed once and record what changed.
    Sync,
    /// Run the HTTP server, polling upstream in the background.
    Serve,
    /// Show recently observed schedule changes.
    Changes {
        #[arg(short, long, default_value_t = 25)]
        limit: i64,
    },
    /// Write a feed to a file from stored state, without serving.
    Export {
        #[arg(short, long)]
        league: String,
        #[arg(short, long)]
        team: String,
        /// Output path; defaults to `<league>-<team>.ics`.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "slate=info".into()))
        .init();

    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;
    let store = Store::connect(&config.database_url).await?;
    store.migrate().await?;

    match cli.command {
        Command::Sync => run_sync(&config, &store).await,
        Command::Serve => run_serve(config, store).await,
        Command::Changes { limit } => run_changes(&store, limit).await,
        Command::Export { league, team, out } => run_export(&store, &league, &team, out).await,
    }
}

async fn run_sync(config: &Config, store: &Store) -> Result<()> {
    let client = espn::Client::new();
    let report = sync_all(config, store, &client).await?;

    println!(
        "{} new, {} changed ({} calendar-significant), {} unchanged",
        report.inserted,
        report.changed.len(),
        report.significant(),
        report.unchanged
    );
    for (game, changes) in &report.changed {
        for change in changes {
            let marker = if change.significant { "*" } else { " " };
            println!(
                "  {marker} {} {}: {} -> {}",
                game.short_name,
                change.field,
                change.old.as_deref().unwrap_or("-"),
                change.new.as_deref().unwrap_or("-")
            );
        }
    }
    Ok(())
}

async fn run_serve(config: Config, store: Store) -> Result<()> {
    let config = Arc::new(config);
    let interval = Duration::from_secs(config.poll_interval_minutes.max(1) * 60);

    let poll_config = Arc::clone(&config);
    let poll_store = store.clone();
    tokio::spawn(async move {
        let client = espn::Client::new();
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            if let Err(error) = sync_all(&poll_config, &poll_store, &client).await {
                tracing::error!(%error, "background sync failed");
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(&config.listen)
        .await
        .with_context(|| format!("binding {}", config.listen))?;
    tracing::info!(address = %config.listen, "slate listening");

    let state = server::AppState { store, config };
    axum::serve(listener, server::router(state)).await?;
    Ok(())
}

async fn run_changes(store: &Store, limit: i64) -> Result<()> {
    let changes = store.recent_changes(limit).await?;
    if changes.is_empty() {
        println!("no changes recorded yet");
        return Ok(());
    }
    for change in &changes {
        let marker = if change.significant { "*" } else { " " };
        println!(
            "{marker} {}  {:<4} {:<14} {:<9} {} -> {}",
            change.observed_at.format("%Y-%m-%d %H:%M"),
            change.league,
            change.short_name,
            change.field,
            change.old_value.as_deref().unwrap_or("-"),
            change.new_value.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

async fn run_export(store: &Store, league: &str, team: &str, out: Option<PathBuf>) -> Result<()> {
    let league = league.to_lowercase();
    let team = store.resolve_team(&league, team).await?.to_lowercase();
    let games = store.feed(&league, &team).await?;
    anyhow::ensure!(
        !games.is_empty(),
        "no stored games for {league}/{team}; run `slate sync` first"
    );

    let bye = {
        let weeks: Vec<i32> = games.iter().filter_map(|f| f.game.week).collect();
        weeks
            .iter()
            .min()
            .zip(weeks.iter().max())
            .and_then(|(first, last)| (*first..=*last).find(|w| !weeks.contains(w)))
    };

    let body = ics::render(&league, &team, &games, bye, Utc::now());
    let path = out.unwrap_or_else(|| PathBuf::from(format!("{league}-{team}.ics")));
    std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
    println!("wrote {} ({} games)", path.display(), games.len());
    Ok(())
}
