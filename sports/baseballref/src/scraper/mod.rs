mod schedule;

use std::{path::Path, time::Duration};

use reqwest::{Client, StatusCode};
pub use schedule::{BoxScoreUrl, extract_boxscore_urls, extract_boxscore_urls_from_html, schedule_url_for_year};
use thiserror::Error;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::{
    db::{BoxScoreInserter, FailedScrapesDb, InsertError},
    parser::BoxScore,
};

const BASE_URL: &str = "https://www.baseball-reference.com";
const USER_AGENT: &str = "Mozilla/5.0 (compatible; BaseballScraper/1.0; educational project)";

/// Base delay between requests (~2 requests/second)
const BASE_DELAY: Duration = Duration::from_secs(3);
/// Maximum delay after repeated backoffs
const MAX_DELAY: Duration = Duration::from_secs(300);
/// Multiplier applied to delay on rate-limit or server errors
const BACKOFF_MULTIPLIER: f64 = 4.0;
/// Maximum retries per request before giving up
const MAX_RETRIES: u32 = 10;

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Rate limited (HTTP {0})")]
    RateLimited(u16),

    #[error("Parse error: {0}")]
    Parse(#[from] crate::parser::ParseError),

    #[error("Insert error: {0}")]
    Insert(#[from] InsertError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of scraping a single box score
#[derive(Debug)]
pub enum ScrapeResult {
    /// Successfully imported
    Imported { game_id: String, db_id: i32 },
    /// Game already exists in database
    AlreadyExists { game_id: String },
    /// Failed to scrape or import
    Failed { game_id: String, error: String },
}

/// Scraper for Baseball Reference box scores
pub struct Scraper {
    client: Client,
    output_dir: Option<std::path::PathBuf>,
}

impl Scraper {
    /// Create a new scraper
    ///
    /// # Panics
    /// Panics if the HTTP client cannot be built.
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("http client should be valid");

        Self {
            client,
            output_dir: None,
        }
    }

    /// Set directory to save downloaded HTML files
    #[must_use]
    pub fn with_output_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.output_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Fetch a schedule page for a given year
    pub async fn fetch_schedule(&self, year: i32) -> Result<String, ScrapeError> {
        let url = schedule_url_for_year(year);
        info!("Fetching schedule: {}", url);

        let response = self.client.get(&url).send().await?;
        let html = response.text().await?;

        // Save to file if output directory is set
        if let Some(ref dir) = self.output_dir {
            let filename = format!("{year}-schedule.shtml");
            let path = dir.join(&filename);
            std::fs::write(&path, &html)?;
            info!("Saved schedule to: {}", path.display());
        }

        Ok(html)
    }

    /// Fetch a box score page, returning the HTML on success or an error with
    /// rate-limit awareness. Returns `ScrapeError::RateLimited` for 429 and 5xx
    /// responses so callers can back off.
    pub async fn fetch_boxscore(&self, url: &BoxScoreUrl) -> Result<String, ScrapeError> {
        let full_url = format!("{}{}", BASE_URL, url.path);
        info!("Fetching: {}", full_url);

        let response = self.client.get(&full_url).send().await?;
        let status = response.status();

        if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
            return Err(ScrapeError::RateLimited(status.as_u16()));
        }

        let html = response.text().await?;

        // Save to file if output directory is set
        if let Some(ref dir) = self.output_dir {
            let filename = format!("{}.shtml", url.game_id);
            let path = dir.join(&filename);
            std::fs::write(&path, &html)?;
            info!("Saved to: {}", path.display());
        }

        Ok(html)
    }

    /// Scrape and import a single box score, retrying with backoff on rate-limit errors.
    /// Returns the result and whether a rate-limit was hit (so the caller can adjust pacing).
    async fn scrape_and_import_with_backoff(
        &self,
        url: &BoxScoreUrl,
        inserter: &BoxScoreInserter<'_>,
        current_delay: &mut Duration,
    ) -> ScrapeResult {
        // Check if game already exists before fetching
        match inserter.game_exists(&url.game_id).await {
            Ok(true) => {
                return ScrapeResult::AlreadyExists {
                    game_id: url.game_id.clone(),
                };
            }
            Ok(false) => {}
            Err(e) => {
                warn!("Failed to check if game exists: {e}, proceeding with fetch");
            }
        }

        let mut attempt = 0;

        let html = loop {
            match self.fetch_boxscore(url).await {
                Ok(h) => {
                    // Success — ease back toward base delay
                    *current_delay = (*current_delay / 2).max(BASE_DELAY);
                    break h;
                }
                Err(ScrapeError::RateLimited(status)) => {
                    attempt += 1;
                    *current_delay = current_delay.mul_f64(BACKOFF_MULTIPLIER).min(MAX_DELAY);
                    if attempt > MAX_RETRIES {
                        return ScrapeResult::Failed {
                            game_id: url.game_id.clone(),
                            error: format!("Rate limited (HTTP {status}) after {MAX_RETRIES} retries"),
                        };
                    }
                    warn!("Rate limited (HTTP {status}), retry {attempt}/{MAX_RETRIES} after {current_delay:?}");
                    sleep(*current_delay).await;
                }
                Err(e) => {
                    return ScrapeResult::Failed {
                        game_id: url.game_id.clone(),
                        error: e.to_string(),
                    };
                }
            }
        };

        // Parse the box score
        let box_score = match BoxScore::from_html(&html, &url.game_id) {
            Ok(bs) => bs,
            Err(e) => {
                return ScrapeResult::Failed {
                    game_id: url.game_id.clone(),
                    error: format!("Parse error: {e}"),
                };
            }
        };

        // Import to database
        match inserter.insert(&box_score).await {
            Ok(db_id) => ScrapeResult::Imported {
                game_id: url.game_id.clone(),
                db_id,
            },
            Err(InsertError::GameExists(id)) => ScrapeResult::AlreadyExists { game_id: id },
            Err(e) => ScrapeResult::Failed {
                game_id: url.game_id.clone(),
                error: format!("Insert error: {e}"),
            },
        }
    }

    /// Scrape multiple box scores with rate limiting
    pub async fn scrape_all(&self, urls: &[BoxScoreUrl], inserter: &BoxScoreInserter<'_>) -> Vec<ScrapeResult> {
        self.scrape_all_with_tracking(urls, inserter, None).await
    }

    /// Scrape multiple box scores with adaptive rate limiting and optional failure tracking.
    ///
    /// Starts at ~2 requests/second and backs off exponentially on 429 / 5xx
    /// errors. After successful requests the delay eases back toward the base rate.
    pub async fn scrape_all_with_tracking(
        &self,
        urls: &[BoxScoreUrl],
        inserter: &BoxScoreInserter<'_>,
        failed_db: Option<&FailedScrapesDb<'_>>,
    ) -> Vec<ScrapeResult> {
        let mut results = Vec::with_capacity(urls.len());
        let total = urls.len();
        let mut delay = BASE_DELAY;

        for (i, url) in urls.iter().enumerate() {
            info!("[{}/{}] Processing: {} (delay: {delay:?})", i + 1, total, url.game_id);

            let result = self.scrape_and_import_with_backoff(url, inserter, &mut delay).await;

            match &result {
                ScrapeResult::Imported { game_id, db_id } => {
                    info!("Imported {} with DB ID {}", game_id, db_id);
                    // Remove from failed scrapes if it was a retry
                    if let Some(db) = failed_db
                        && let Err(e) = db.delete_failure(game_id).await
                    {
                        warn!("Failed to remove {} from failed_scrapes: {}", game_id, e);
                    }
                }
                ScrapeResult::AlreadyExists { game_id } => {
                    info!("Skipped {} (already exists)", game_id);
                    // Also remove from failed scrapes since it exists
                    if let Some(db) = failed_db
                        && let Err(e) = db.delete_failure(game_id).await
                    {
                        warn!("Failed to remove {} from failed_scrapes: {}", game_id, e);
                    }
                }
                ScrapeResult::Failed { game_id, error } => {
                    warn!("Failed {}: {}", game_id, error);
                    // Record the failure
                    if let Some(db) = failed_db
                        && let Err(e) = db.record_failure(game_id, error).await
                    {
                        warn!("Failed to record failure for {}: {}", game_id, e);
                    }
                }
            }

            // Only delay after actual HTTP requests (not skipped duplicates)
            let needs_delay = !matches!(&result, ScrapeResult::AlreadyExists { .. });

            results.push(result);

            if needs_delay && i < total - 1 {
                sleep(delay).await;
            }
        }

        results
    }
}

impl Default for Scraper {
    fn default() -> Self {
        Self::new()
    }
}
