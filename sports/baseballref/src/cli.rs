use std::path::PathBuf;

use clap::Subcommand;
use tracing::{error, info};

use crate::{
    db::{BoxScoreInserter, create_pool, run_migrations},
    parser::BoxScore,
    scraper::{BoxScoreUrl, ScrapeResult, Scraper, extract_boxscore_urls},
};

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
}

/// # Panics
///
/// Can panic.
pub async fn handle_command(command: BaseballCommands) -> anyhow::Result<()> {
    match command {
        BaseballCommands::Parse { file } => {
            info!("Parsing file: {}", file.display());

            let box_score = BoxScore::from_file(&file)?;
            println!("{}", box_score.summary());

            // Print more details
            println!("\n--- Batting Lines ---");
            for line in &box_score.batting_lines {
                if let Some(order) = line.batting_order {
                    println!(
                        "{order}. {} ({}) - {}/{}, {} RBI",
                        line.player_name,
                        line.position.as_deref().unwrap_or("?"),
                        line.h.unwrap_or(0),
                        line.ab.unwrap_or(0),
                        line.rbi.unwrap_or(0)
                    );
                }
            }

            println!("\n--- Pitching Lines ---");
            for line in &box_score.pitching_lines {
                println!(
                    "{}. {} {} - {} IP, {} K, {} ER",
                    line.pitch_order,
                    line.player_name,
                    line.decision.as_deref().unwrap_or(""),
                    line.ip.map(|ip| ip.to_string()).unwrap_or_default(),
                    line.so.unwrap_or(0),
                    line.er.unwrap_or(0)
                );
            }

            println!("\n--- Play-by-Play ({} events) ---", box_score.play_by_play.len());
            for event in box_score.play_by_play.iter().take(5) {
                let half = if event.is_bottom { "Bot" } else { "Top" };
                println!(
                    "{} {} - {} vs {} - {}",
                    half,
                    event.inning,
                    event.batter_name,
                    event.pitcher_name,
                    event.play_description.as_deref().unwrap_or("?")
                );
            }
            if box_score.play_by_play.len() > 5 {
                println!("... and {} more events", box_score.play_by_play.len() - 5);
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
                    println!("Successfully imported: {}", box_score.game_info.bbref_game_id);
                    println!("Database game ID: {game_id}");
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
            println!("Migrations applied successfully");
        }

        BaseballCommands::ListGames { schedule, limit } => {
            info!("Extracting box score URLs from: {}", schedule.display());

            let urls = extract_boxscore_urls(&schedule)?;
            let today = chrono::Utc::now().format("%Y%m%d").to_string();

            println!("Found {} box score URLs", urls.len());
            println!();

            let urls_to_show: Vec<_> = urls
                .iter()
                .filter(|u| {
                    // Extract date from game ID (e.g., CHN202503180 -> 20250318)
                    if u.game_id.len() >= 11 {
                        let date = &u.game_id[3..11];
                        date <= today.as_str()
                    } else {
                        true
                    }
                })
                .take(limit.unwrap_or(usize::MAX))
                .collect();

            for url in &urls_to_show {
                println!("{}: {}", url.game_id, url.path);
            }

            if let Some(n) = limit
                && urls.len() > n
            {
                println!("... and {} more", urls.len() - n);
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
                println!("No .shtml files found in {}", input_dir.display());
                return Ok(());
            }

            // Apply limit if specified
            if let Some(n) = limit {
                files.truncate(n);
            }

            println!("Found {} files to import\n", files.len());

            if dry_run {
                for file in &files {
                    println!("  {}", file.file_name().unwrap().to_string_lossy());
                }
                println!("\nWould import {} files (dry run)", files.len());
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
                        println!(
                            "✗ {}: Parse error: {}",
                            file_path.file_name().unwrap().to_string_lossy(),
                            e
                        );
                        fail_count += 1;
                        continue;
                    }
                };

                // If force flag is set, delete existing game first
                if force {
                    let _ = sqlx::query("DELETE FROM games WHERE bbref_game_id = $1")
                        .bind(&box_score.game_info.bbref_game_id)
                        .execute(&pool)
                        .await;
                }

                // Insert into database
                let inserter = BoxScoreInserter::new(&pool);
                match inserter.insert(&box_score).await {
                    Ok(_) => {
                        println!("✓ {}", file_path.file_name().unwrap().to_string_lossy());
                        success_count += 1;
                    }
                    Err(crate::db::InsertError::GameExists(_)) => {
                        println!(
                            "✓ {} (already exists)",
                            file_path.file_name().unwrap().to_string_lossy()
                        );
                        success_count += 1;
                    }
                    Err(e) => {
                        println!("✗ {}: {}", file_path.file_name().unwrap().to_string_lossy(), e);
                        fail_count += 1;
                    }
                }
            }

            // Summary
            println!();
            println!("{}", "=".repeat(50));
            println!("Success: {success_count}");
            println!("Failed:  {fail_count}");
            println!("Total:   {total}");
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
                println!("No game IDs provided");
                return Ok(());
            }

            println!("Found {} game IDs to retry\n", ids.len());

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
                Scraper::new()?.with_output_dir(dir)
            } else {
                Scraper::new()?
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

            // Summary
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

            println!();
            println!("=== Retry Summary ===");
            println!("Imported: {imported}");
            println!("Skipped (already exists): {skipped}");
            println!("Failed: {failed}");

            if failed > 0 {
                println!();
                println!("Still failed:");
                for result in &results {
                    if let ScrapeResult::Failed { game_id, error } = result {
                        println!("  {game_id}: {error}");
                    }
                }
            }
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

            // Filter future games if requested
            if skip_future {
                let today = chrono::Utc::now().format("%Y%m%d").to_string();
                let original_count = urls.len();
                urls.retain(|u| {
                    if u.game_id.len() >= 11 {
                        let date = &u.game_id[3..11];
                        date <= today.as_str()
                    } else {
                        true
                    }
                });
                info!(
                    "Filtered to {} past/current games (skipped {} future)",
                    urls.len(),
                    original_count - urls.len()
                );
            }

            // Apply limit if specified
            if let Some(n) = limit {
                urls.truncate(n);
                info!("Limited to {} games", urls.len());
            }

            if urls.is_empty() {
                println!("No games to scrape");
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
                Scraper::new()?.with_output_dir(dir)
            } else {
                Scraper::new()?
            };

            // Create inserter
            let inserter = BoxScoreInserter::new(&pool);

            // Scrape all games
            let results = scraper.scrape_all(&urls, &inserter).await;

            // Summary
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

            println!();
            println!("=== Scrape Summary ===");
            println!("Imported: {imported}");
            println!("Skipped (already exists): {skipped}");
            println!("Failed: {failed}");

            if failed > 0 {
                println!();
                println!("Failed games:");
                for result in &results {
                    if let ScrapeResult::Failed { game_id, error } = result {
                        println!("  {game_id}: {error}");
                    }
                }
            }
        }
    }

    Ok(())
}
