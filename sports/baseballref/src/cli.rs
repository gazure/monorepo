use std::path::PathBuf;

use clap::Subcommand;
use tracing::{error, info, warn};

use crate::{
    db::{BoxScoreInserter, FailedScrapesDb, create_pool, run_migrations},
    parser::BoxScore,
    scraper::{BoxScoreUrl, ScrapeResult, Scraper, extract_boxscore_urls, extract_boxscore_urls_from_html},
};

/// Extracts the date portion (YYYYMMDD) from a game ID like "CHN202503180".
/// Returns None if the game ID is too short.
fn game_id_date(game_id: &str) -> Option<&str> {
    // Game IDs are formatted as: {3-char team code}{YYYYMMDD}{game number}
    (game_id.len() >= 11).then(|| &game_id[3..11])
}

/// Returns true if the game is in the past or present (not future).
fn is_past_game(url: &BoxScoreUrl) -> bool {
    let today = chrono::Utc::now().format("%Y%m%d").to_string();
    game_id_date(&url.game_id).is_none_or(|date| date <= today.as_str())
}

/// Filter out future games and apply an optional limit.
fn filter_and_limit(urls: &mut Vec<BoxScoreUrl>, skip_future: bool, limit: Option<usize>) {
    if skip_future {
        let original_count = urls.len();
        urls.retain(is_past_game);
        info!(
            "Filtered to {} past/current games (skipped {} future)",
            urls.len(),
            original_count - urls.len()
        );
    }

    if let Some(n) = limit {
        urls.truncate(n);
        info!("Limited to {} games", urls.len());
    }
}

struct ScrapeSummary {
    imported: usize,
    skipped: usize,
    failed: usize,
}

/// Count results by category and log a summary with failed game details.
fn summarize_results(results: &[ScrapeResult], title: &str) -> ScrapeSummary {
    let imported = results
        .iter()
        .filter(|r| matches!(r, ScrapeResult::Imported { .. }))
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r, ScrapeResult::AlreadyExists { .. }))
        .count();
    let failed = results
        .iter()
        .filter(|r| matches!(r, ScrapeResult::Failed { .. }))
        .count();

    info!("");
    info!("=== {title} ===");
    info!("Imported: {imported}");
    info!("Skipped (already exists): {skipped}");
    info!("Failed: {failed}");

    if failed > 0 {
        info!("");
        info!("Failed games:");
        for result in results {
            if let ScrapeResult::Failed { game_id, error } = result {
                info!("  {game_id}: {error}");
            }
        }
    }

    ScrapeSummary {
        imported,
        skipped,
        failed,
    }
}

#[derive(Subcommand)]
pub enum BaseballCommands {
    /// Parse a box score file and print a summary
    Parse {
        /// Path to the HTML file
        file: PathBuf,
    },

    /// Parse a box score file and import it to the database
    Import {
        /// Path to the HTML file
        file: PathBuf,

        /// Database URL (or set `SPORTS_DATABASE_URL` env var)
        #[arg(short, long, env = "SPORTS_DATABASE_URL")]
        database_url: String,

        /// Force re-import by deleting existing game data first
        #[arg(short, long)]
        force: bool,
    },

    /// Run database migrations
    Migrate {
        /// Database URL (or set `SPORTS_DATABASE_URL` env var)
        #[arg(short, long, env = "SPORTS_DATABASE_URL")]
        database_url: String,
    },

    /// Scrape box scores from a schedule file
    Scrape {
        /// Path to the schedule HTML file
        schedule: PathBuf,

        /// Database URL (or set `SPORTS_DATABASE_URL` env var)
        #[arg(short, long, env = "SPORTS_DATABASE_URL")]
        database_url: String,

        /// Directory to save downloaded HTML files
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Maximum number of games to scrape (for testing)
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Skip games that are in the future (based on game ID date)
        #[arg(long, default_value = "true")]
        skip_future: bool,
    },

    /// List box score URLs from a schedule file (dry run)
    ListGames {
        /// Path to the schedule HTML file
        schedule: PathBuf,

        /// Maximum number of games to list
        #[arg(short = 'n', long)]
        limit: Option<usize>,
    },

    /// Scrape box scores from schedule URLs by year range
    ScrapeYears {
        /// Start year (inclusive)
        #[arg(short = 's', long)]
        start_year: i32,

        /// End year (inclusive, defaults to start year)
        #[arg(short = 'e', long)]
        end_year: Option<i32>,

        /// Database URL (or set `SPORTS_DATABASE_URL` env var)
        #[arg(short, long, env = "SPORTS_DATABASE_URL")]
        database_url: String,

        /// Directory to save downloaded HTML files
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Maximum number of games to scrape per year (for testing)
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Skip games that are in the future (based on game ID date)
        #[arg(long, default_value = "true")]
        skip_future: bool,
    },

    /// Retry importing all scraped games from a directory
    RetryImports {
        /// Directory containing .shtml files
        #[arg(short = 'i', long, default_value = "sports/data/bbref")]
        input_dir: PathBuf,

        /// Database URL (or set `SPORTS_DATABASE_URL` env var)
        #[arg(short, long, env = "SPORTS_DATABASE_URL")]
        database_url: String,

        /// Maximum number of files to import
        #[arg(short = 'n', long)]
        limit: Option<usize>,

        /// Force re-import by deleting existing games first
        #[arg(short, long)]
        force: bool,

        /// List files without importing (dry run)
        #[arg(long)]
        dry_run: bool,
    },

    /// Retry scraping failed games by game ID
    RetryFailed {
        /// File containing game IDs (one per line) or comma-separated game IDs
        #[arg(short = 'g', long)]
        game_ids: String,

        /// Database URL (or set `SPORTS_DATABASE_URL` env var)
        #[arg(short, long, env = "SPORTS_DATABASE_URL")]
        database_url: String,

        /// Directory to save downloaded HTML files
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Force re-import by deleting existing games first
        #[arg(short, long)]
        force: bool,
    },

    /// List all failed scrapes stored in the database
    FailedList {
        /// Database URL (or set `SPORTS_DATABASE_URL` env var)
        #[arg(short, long, env = "SPORTS_DATABASE_URL")]
        database_url: String,
    },

    /// Delete failed scrape records from the database
    FailedDelete {
        /// Game ID to delete, or "all" to clear all failed scrapes
        #[arg(short = 'g', long)]
        game_id: String,

        /// Database URL (or set `SPORTS_DATABASE_URL` env var)
        #[arg(short, long, env = "SPORTS_DATABASE_URL")]
        database_url: String,
    },

    /// Retry all failed scrapes stored in the database
    FailedRetry {
        /// Database URL (or set `SPORTS_DATABASE_URL` env var)
        #[arg(short, long, env = "SPORTS_DATABASE_URL")]
        database_url: String,

        /// Directory to save downloaded HTML files
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Force re-import by deleting existing games first
        #[arg(short, long)]
        force: bool,

        /// Maximum number of games to retry
        #[arg(short = 'n', long)]
        limit: Option<usize>,
    },
}

/// # Panics
///
/// Can panic.
pub async fn handle_command(command: BaseballCommands) -> anyhow::Result<()> {
    match command {
        BaseballCommands::Parse { file } => {
            info!("Parsing file: {}", file.display());

            let box_score = BoxScore::from_file(&file)?;
            info!("{}", box_score.summary());

            // Print more details
            info!("\n--- Batting Lines ---");
            for line in &box_score.batting_lines {
                if let Some(order) = line.batting_order {
                    info!(
                        "{order}. {} ({}) - {}/{}, {} RBI",
                        line.player_name,
                        line.position.as_deref().unwrap_or("?"),
                        line.h.unwrap_or(0),
                        line.ab.unwrap_or(0),
                        line.rbi.unwrap_or(0)
                    );
                }
            }

            info!("\n--- Pitching Lines ---");
            for line in &box_score.pitching_lines {
                info!(
                    "{}. {} {} - {} IP, {} K, {} ER",
                    line.pitch_order,
                    line.player_name,
                    line.decision.as_deref().unwrap_or(""),
                    line.ip.map(|ip| ip.to_string()).unwrap_or_default(),
                    line.so.unwrap_or(0),
                    line.er.unwrap_or(0)
                );
            }

            info!("\n--- Play-by-Play ({} events) ---", box_score.play_by_play.len());
            for event in box_score.play_by_play.iter().take(5) {
                let half = if event.is_bottom { "Bot" } else { "Top" };
                info!(
                    "{} {} - {} vs {} - {}",
                    half,
                    event.inning,
                    event.batter_name,
                    event.pitcher_name,
                    event.play_description.as_deref().unwrap_or("?")
                );
            }
            if box_score.play_by_play.len() > 5 {
                info!("... and {} more events", box_score.play_by_play.len() - 5);
            }
        }

        BaseballCommands::Import {
            file,
            database_url,
            force,
        } => {
            info!("Importing file: {}", file.display());

            // Connect to database
            let pool = create_pool(&database_url).await?;
            info!("Connected to database");

            // Run migrations
            run_migrations(&pool).await?;
            info!("Migrations complete");

            // Parse the file
            let box_score = BoxScore::from_file(&file)?;
            info!("Parsed box score: {}", box_score.game_info.bbref_game_id);

            // If force flag is set, delete existing game first
            if force {
                let result = sqlx::query("DELETE FROM games WHERE bbref_game_id = $1")
                    .bind(&box_score.game_info.bbref_game_id)
                    .execute(&pool)
                    .await?;

                if result.rows_affected() > 0 {
                    info!("Deleted existing game: {}", box_score.game_info.bbref_game_id);
                }
            }

            // Insert into database
            let inserter = BoxScoreInserter::new(&pool);
            match inserter.insert(&box_score).await {
                Ok(game_id) => {
                    info!("Successfully imported game with ID: {game_id}");
                    info!("Successfully imported: {}", box_score.game_info.bbref_game_id);
                    info!("Database game ID: {game_id}");
                }
                Err(e) => {
                    error!("Failed to import: {e}");
                    return Err(e.into());
                }
            }
        }

        BaseballCommands::Migrate { database_url } => {
            info!("Running migrations");

            let pool = create_pool(&database_url).await?;
            run_migrations(&pool).await?;

            info!("Migrations complete");
            info!("Migrations applied successfully");
        }

        BaseballCommands::ListGames { schedule, limit } => {
            info!("Extracting box score URLs from: {}", schedule.display());

            let mut urls = extract_boxscore_urls(&schedule)?;
            info!("Found {} box score URLs", urls.len());

            filter_and_limit(&mut urls, true, limit);

            info!("");
            for url in &urls {
                info!("{}: {}", url.game_id, url.path);
            }
        }

        BaseballCommands::RetryImports {
            input_dir,
            database_url,
            limit,
            force,
            dry_run,
        } => {
            info!("Retrying imports from: {}", input_dir.display());

            // Find all .shtml files (excluding schedule files)
            let mut files: Vec<PathBuf> = std::fs::read_dir(&input_dir)?
                .filter_map(std::result::Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension().is_some_and(|ext| ext == "shtml")
                        && !path.file_name().unwrap().to_string_lossy().ends_with("-schedule.shtml")
                })
                .collect();

            files.sort();

            if files.is_empty() {
                info!("No .shtml files found in {}", input_dir.display());
                return Ok(());
            }

            // Apply limit if specified
            if let Some(n) = limit {
                files.truncate(n);
            }

            info!("Found {} files to import\n", files.len());

            if dry_run {
                for file in &files {
                    info!("  {}", file.file_name().unwrap().to_string_lossy());
                }
                info!("\nWould import {} files (dry run)", files.len());
                return Ok(());
            }

            // Connect to database
            let pool = create_pool(&database_url).await?;
            info!("Connected to database");

            // Run migrations
            run_migrations(&pool).await?;
            info!("Migrations complete");

            // Import all files
            let mut success_count = 0;
            let mut fail_count = 0;
            let total = files.len();

            for (i, file_path) in files.iter().enumerate() {
                print!("[{}/{}] ", i + 1, total);

                // Parse the file
                let box_score = match BoxScore::from_file(file_path) {
                    Ok(bs) => bs,
                    Err(e) => {
                        info!(
                            "✗ {}: Parse error: {}",
                            file_path.file_name().unwrap().to_string_lossy(),
                            e
                        );
                        fail_count += 1;
                        continue;
                    }
                };

                // If force flag is set, delete existing game first
                if force
                    && let Err(e) = sqlx::query("DELETE FROM games WHERE bbref_game_id = $1")
                        .bind(&box_score.game_info.bbref_game_id)
                        .execute(&pool)
                        .await
                {
                    warn!(
                        "Failed to delete existing game {}: {e}",
                        box_score.game_info.bbref_game_id
                    );
                }

                // Insert into database
                let inserter = BoxScoreInserter::new(&pool);
                match inserter.insert(&box_score).await {
                    Ok(_) => {
                        info!("✓ {}", file_path.file_name().unwrap().to_string_lossy());
                        success_count += 1;
                    }
                    Err(crate::db::InsertError::GameExists(_)) => {
                        info!(
                            "✓ {} (already exists)",
                            file_path.file_name().unwrap().to_string_lossy()
                        );
                        success_count += 1;
                    }
                    Err(e) => {
                        info!("✗ {}: {}", file_path.file_name().unwrap().to_string_lossy(), e);
                        fail_count += 1;
                    }
                }
            }

            // Summary
            info!("");
            info!("{}", "=".repeat(50));
            info!("Success: {success_count}");
            info!("Failed:  {fail_count}");
            info!("Total:   {total}");
        }

        BaseballCommands::RetryFailed {
            game_ids,
            database_url,
            output_dir,
            force,
        } => {
            info!("Retrying failed games");

            // Parse game IDs from file or comma-separated string
            let ids: Vec<String> = if std::path::Path::new(&game_ids).exists() {
                // Read from file
                std::fs::read_to_string(&game_ids)?
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .map(|line| {
                        // Extract just the game ID if line contains ": error..."
                        line.split(':').next().unwrap_or(line).trim().to_string()
                    })
                    .collect()
            } else {
                // Parse as comma-separated
                game_ids.split(',').map(|s| s.trim().to_string()).collect()
            };

            if ids.is_empty() {
                info!("No game IDs provided");
                return Ok(());
            }

            info!("Found {} game IDs to retry\n", ids.len());

            // Convert game IDs to BoxScoreUrl format
            let urls: Vec<BoxScoreUrl> = ids
                .iter()
                .map(|id| BoxScoreUrl {
                    game_id: id.clone(),
                    path: format!("/boxes/{}/{}.shtml", &id[..3], id),
                })
                .collect();

            // Connect to database
            let pool = create_pool(&database_url).await?;
            info!("Connected to database");

            // Run migrations
            run_migrations(&pool).await?;
            info!("Migrations complete");

            // Create scraper
            let scraper = if let Some(ref dir) = output_dir {
                std::fs::create_dir_all(dir)?;
                Scraper::new().with_output_dir(dir)
            } else {
                Scraper::new()
            };

            // Create inserter
            let inserter = BoxScoreInserter::new(&pool);

            // If force is set, we need to delete games before scraping
            if force {
                for game_id in &ids {
                    let result = sqlx::query("DELETE FROM games WHERE bbref_game_id = $1")
                        .bind(game_id)
                        .execute(&pool)
                        .await?;
                    if result.rows_affected() > 0 {
                        info!("Deleted existing game: {}", game_id);
                    }
                }
            }

            // Scrape all games
            let results = scraper.scrape_all(&urls, &inserter).await;

            summarize_results(&results, "Retry Summary");
        }

        BaseballCommands::Scrape {
            schedule,
            database_url,
            output_dir,
            limit,
            skip_future,
        } => {
            info!("Scraping box scores from: {}", schedule.display());

            // Extract URLs from schedule
            let mut urls = extract_boxscore_urls(&schedule)?;
            info!("Found {} box score URLs", urls.len());

            filter_and_limit(&mut urls, skip_future, limit);

            if urls.is_empty() {
                info!("No games to scrape");
                return Ok(());
            }

            // Connect to database
            let pool = create_pool(&database_url).await?;
            info!("Connected to database");

            // Run migrations
            run_migrations(&pool).await?;
            info!("Migrations complete");

            // Create scraper
            let scraper = if let Some(ref dir) = output_dir {
                // Create output directory if it doesn't exist
                std::fs::create_dir_all(dir)?;
                Scraper::new().with_output_dir(dir)
            } else {
                Scraper::new()
            };

            // Create inserter
            let inserter = BoxScoreInserter::new(&pool);

            // Create failed scrapes tracker
            let failed_db = FailedScrapesDb::new(&pool);

            // Scrape all games with failure tracking
            let results = scraper
                .scrape_all_with_tracking(&urls, &inserter, Some(&failed_db))
                .await;

            summarize_results(&results, "Scrape Summary");
        }

        BaseballCommands::ScrapeYears {
            start_year,
            end_year,
            database_url,
            output_dir,
            limit,
            skip_future,
        } => {
            let end_year = end_year.unwrap_or(start_year);

            if start_year > end_year {
                return Err(anyhow::anyhow!("Start year must be <= end year"));
            }

            info!("Scraping years {start_year} to {end_year}");

            // Connect to database
            let pool = create_pool(&database_url).await?;
            info!("Connected to database");

            // Run migrations
            run_migrations(&pool).await?;
            info!("Migrations complete");

            // Create scraper
            let scraper = if let Some(ref dir) = output_dir {
                std::fs::create_dir_all(dir)?;
                Scraper::new().with_output_dir(dir)
            } else {
                Scraper::new()
            };

            // Create inserter
            let inserter = BoxScoreInserter::new(&pool);

            // Create failed scrapes tracker
            let failed_db = FailedScrapesDb::new(&pool);

            // Track overall statistics
            let mut total_imported = 0;
            let mut total_skipped = 0;
            let mut total_failed = 0;

            // Process each year
            for year in start_year..=end_year {
                info!("");
                info!("=== Year {year} ===");

                // Fetch schedule page
                let schedule_html = match scraper.fetch_schedule(year).await {
                    Ok(html) => html,
                    Err(e) => {
                        error!("Failed to fetch schedule for {year}: {e}");
                        info!("Failed to fetch schedule for {year}: {e}");
                        continue;
                    }
                };

                // Extract box score URLs
                let mut urls = extract_boxscore_urls_from_html(&schedule_html);
                info!("Found {} box score URLs for {year}", urls.len());

                filter_and_limit(&mut urls, skip_future, limit);

                if urls.is_empty() {
                    info!("No games to scrape for {year}");
                    continue;
                }

                info!("Scraping {} games from {year}", urls.len());

                // Scrape all games for this year with failure tracking
                let results = scraper
                    .scrape_all_with_tracking(&urls, &inserter, Some(&failed_db))
                    .await;

                let summary = summarize_results(&results, &format!("Year {year} Summary"));
                total_imported += summary.imported;
                total_skipped += summary.skipped;
                total_failed += summary.failed;
            }

            // Overall summary
            info!("");
            info!("{}", "=".repeat(50));
            info!("=== Overall Summary ({start_year}-{end_year}) ===");
            info!("Total Imported: {total_imported}");
            info!("Total Skipped: {total_skipped}");
            info!("Total Failed: {total_failed}");
            if total_failed > 0 {
                info!("\nFailed games saved for retry with `failed-retry` command.");
            }
        }

        BaseballCommands::FailedList { database_url } => {
            // Connect to database
            let pool = create_pool(&database_url).await?;
            run_migrations(&pool).await?;

            let failed_db = FailedScrapesDb::new(&pool);
            let failures = failed_db.list_failures().await?;

            if failures.is_empty() {
                info!("No failed scrapes recorded.");
                return Ok(());
            }

            info!("Failed Scrapes ({} games):\n", failures.len());
            for failure in &failures {
                let time = failure.failed_at.format("%Y-%m-%d %H:%M:%S");
                let attempts = if failure.attempt_count == 1 {
                    "1 attempt".to_string()
                } else {
                    format!("{} attempts", failure.attempt_count)
                };
                info!(
                    "  {}: {} (failed {}, {})",
                    failure.bbref_game_id, failure.error_message, time, attempts
                );
            }
        }

        BaseballCommands::FailedDelete { game_id, database_url } => {
            // Connect to database
            let pool = create_pool(&database_url).await?;
            run_migrations(&pool).await?;

            let failed_db = FailedScrapesDb::new(&pool);

            if game_id == "all" {
                let count = failed_db.clear_failures().await?;
                info!("Deleted {count} failed scrape record(s).");
            } else {
                let deleted = failed_db.delete_failure(&game_id).await?;
                if deleted {
                    info!("Deleted failed scrape record for {game_id}.");
                } else {
                    info!("No failed scrape record found for {game_id}.");
                }
            }
        }

        BaseballCommands::FailedRetry {
            database_url,
            output_dir,
            force,
            limit,
        } => {
            // Connect to database
            let pool = create_pool(&database_url).await?;
            run_migrations(&pool).await?;

            let failed_db = FailedScrapesDb::new(&pool);
            let mut failures = failed_db.list_failures().await?;

            if failures.is_empty() {
                info!("No failed scrapes to retry.");
                return Ok(());
            }

            // Apply limit if specified
            if let Some(n) = limit {
                failures.truncate(n);
            }

            info!("Retrying {} failed games...\n", failures.len());

            // Convert to BoxScoreUrl format
            let urls: Vec<BoxScoreUrl> = failures
                .iter()
                .map(|f| BoxScoreUrl {
                    game_id: f.bbref_game_id.clone(),
                    path: format!("/boxes/{}/{}.shtml", &f.bbref_game_id[..3], f.bbref_game_id),
                })
                .collect();

            // Create scraper
            let scraper = if let Some(ref dir) = output_dir {
                std::fs::create_dir_all(dir)?;
                Scraper::new().with_output_dir(dir)
            } else {
                Scraper::new()
            };

            // Create inserter
            let inserter = BoxScoreInserter::new(&pool);

            // If force is set, delete existing games before scraping
            if force {
                for failure in &failures {
                    let result = sqlx::query("DELETE FROM games WHERE bbref_game_id = $1")
                        .bind(&failure.bbref_game_id)
                        .execute(&pool)
                        .await?;
                    if result.rows_affected() > 0 {
                        info!("Deleted existing game: {}", failure.bbref_game_id);
                    }
                }
            }

            // Scrape all games with failure tracking
            let results = scraper
                .scrape_all_with_tracking(&urls, &inserter, Some(&failed_db))
                .await;

            summarize_results(&results, "Retry Summary");
        }
    }

    Ok(())
}
