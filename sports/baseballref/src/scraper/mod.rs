mod client;
mod schedule;

pub use client::{ScrapeError, ScrapeResult, Scraper};
pub use schedule::{BoxScoreUrl, extract_boxscore_urls, extract_boxscore_urls_from_html, schedule_url_for_year};
