use anyhow::Result;

use super::Fetcher;
use crate::models::{DeckCard, parse_deck_download};

/// Fetch and parse a deck's card list from the download endpoint.
pub async fn fetch_deck_cards(fetcher: &Fetcher, goldfish_deck_id: i32) -> Result<Vec<DeckCard>> {
    let text = fetcher.fetch(&format!("/deck/download/{goldfish_deck_id}")).await?;
    Ok(parse_deck_download(&text))
}
