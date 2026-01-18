#![expect(clippy::cast_possible_truncation)]
#![expect(clippy::cast_possible_wrap)]
#![expect(clippy::too_many_lines)]
pub mod cli;
pub mod db;
pub mod models;
pub mod parser;
pub mod scraper;
