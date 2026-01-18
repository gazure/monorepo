mod schedule;

use std::{path::Path, time::Duration};

use reqwest::Client;
pub use schedule::{BoxScoreUrl, extract_boxscore_urls};
use thiserror::Error;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::{
    db::{BoxScoreInserter, InsertError},
    parser::BoxScore,
};

const BASE_URL: &str = "https://www.baseball-reference.com";
const USER_AGENT: &str = "Mozilla/5.0 (compatible; BaseballScraper/1.0; educational project)";

/// Delay between requests to respect rate limiting (10 requests/minute = 6 seconds between requests)
const REQUEST_DELAY: Duration = Duration::from_secs(7);

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

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
    pub fn new() -> Result<Self, ScrapeError> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client,
            output_dir: None,
        })
    }

    /// Set directory to save downloaded HTML files
    #[must_use]
    pub fn with_output_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.output_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Fetch a box score page
    pub async fn fetch_boxscore(&self, url: &BoxScoreUrl) -> Result<String, ScrapeError> {
        let full_url = format!("{}{}", BASE_URL, url.path);
        info!("Fetching: {}", full_url);

        let response = self.client.get(&full_url).send().await?;
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

    /// Scrape and import a single box score
    pub async fn scrape_and_import(&self, url: &BoxScoreUrl, inserter: &BoxScoreInserter<'_>) -> ScrapeResult {
        // Fetch the HTML
        let html = match self.fetch_boxscore(url).await {
            Ok(h) => h,
            Err(e) => {
                return ScrapeResult::Failed {
                    game_id: url.game_id.clone(),
                    error: e.to_string(),
                };
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
        let mut results = Vec::with_capacity(urls.len());
        let total = urls.len();

        for (i, url) in urls.iter().enumerate() {
            info!("[{}/{}] Processing: {}", i + 1, total, url.game_id);

            let result = self.scrape_and_import(url, inserter).await;

            match &result {
                ScrapeResult::Imported { game_id, db_id } => {
                    info!("Imported {} with DB ID {}", game_id, db_id);
                }
                ScrapeResult::AlreadyExists { game_id } => {
                    info!("Skipped {} (already exists)", game_id);
                }
                ScrapeResult::Failed { game_id, error } => {
                    warn!("Failed {}: {}", game_id, error);
                }
            }

            results.push(result);

            // Rate limiting - wait between requests (except for last one)
            if i < total - 1 {
                info!("Waiting {:?} before next request...", REQUEST_DELAY);
                sleep(REQUEST_DELAY).await;
            }
        }

        results
    }
}

impl Default for Scraper {
    fn default() -> Self {
        Self::new().expect("Failed to create default scraper")
    }
}
