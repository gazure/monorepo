use chrono::{Datelike};

use crate::giftexchange::ExchangePool;

/// Returns a letter identifier for the given exchange pool
///
/// - IslandLife always returns 'I'
/// - Pets always returns 'P'
/// - Grabergishimazureson returns a random letter from a predefined set
pub fn letter_for_pool(pool: ExchangePool) -> char {
    let letters = "ACDIJLMNORSTUXYZ".chars().collect::<Vec<char>>();

    match pool {
        ExchangePool::IslandLife => 'I',
        ExchangePool::Grabergishimazureson => *fastrand::choice(letters.iter()).unwrap(),
        ExchangePool::Pets => 'P',
    }
}

/// Returns the current year
pub fn current_year() -> i32 {
    chrono::Utc::now().year()
}
