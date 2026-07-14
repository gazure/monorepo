use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Batting line for a player in a game
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BattingLine {
    pub id: i32,
    pub game_id: i32,
    pub player_id: i32,
    pub team_id: i32,
    pub batting_order: Option<i32>,
    pub position: Option<String>,
    pub ab: Option<i32>,
    pub r: Option<i32>,
    pub h: Option<i32>,
    pub rbi: Option<i32>,
    pub bb: Option<i32>,
    pub so: Option<i32>,
    pub pa: Option<i32>,
    pub batting_avg: Option<Decimal>,
    pub obp: Option<Decimal>,
    pub slg: Option<Decimal>,
    pub ops: Option<Decimal>,
    pub pitches_seen: Option<i32>,
    pub strikes_seen: Option<i32>,
    pub wpa: Option<Decimal>,
    pub ali: Option<Decimal>,
    pub wpa_pos: Option<Decimal>,
    pub wpa_neg: Option<Decimal>,
    pub cwpa: Option<Decimal>,
    pub acli: Option<Decimal>,
    pub re24: Option<Decimal>,
    pub po: Option<i32>,
    pub a: Option<i32>,
    pub details: Option<String>,
}

/// Counting stats parsed from the box-score `details` tag string
/// ("HR,2·2B,SB"; the multiplier separator is U+00B7). The vocabulary is
/// closed: 2B, 3B, HR, SB, CS, GDP, SF, SH, HBP, IW.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailCounts {
    pub doubles: i32,
    pub triples: i32,
    pub home_runs: i32,
    pub stolen_bases: i32,
    pub caught_stealing: i32,
    pub gdp: i32,
    pub sac_flies: i32,
    pub sac_hits: i32,
    pub hbp: i32,
    pub ibb: i32,
}

impl DetailCounts {
    pub fn parse(details: Option<&str>) -> Self {
        let mut counts = Self::default();
        let Some(details) = details else {
            return counts;
        };
        for item in details.split(',') {
            let item = item.trim();
            let (n, tag) = match item.split_once('·') {
                Some((n, tag)) => (n.parse().unwrap_or(1), tag),
                None => (1, item),
            };
            match tag {
                "2B" => counts.doubles += n,
                "3B" => counts.triples += n,
                "HR" => counts.home_runs += n,
                "SB" => counts.stolen_bases += n,
                "CS" => counts.caught_stealing += n,
                "GDP" => counts.gdp += n,
                "SF" => counts.sac_flies += n,
                "SH" => counts.sac_hits += n,
                "HBP" => counts.hbp += n,
                "IW" => counts.ibb += n,
                _ => {}
            }
        }
        counts
    }
}

/// Batting line data for insertion
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewBattingLine {
    pub game_id: i32,
    pub player_id: i32,
    pub team_id: i32,
    pub batting_order: Option<i32>,
    pub position: Option<String>,
    pub ab: Option<i32>,
    pub r: Option<i32>,
    pub h: Option<i32>,
    pub rbi: Option<i32>,
    pub bb: Option<i32>,
    pub so: Option<i32>,
    pub pa: Option<i32>,
    pub batting_avg: Option<Decimal>,
    pub obp: Option<Decimal>,
    pub slg: Option<Decimal>,
    pub ops: Option<Decimal>,
    pub pitches_seen: Option<i32>,
    pub strikes_seen: Option<i32>,
    pub wpa: Option<Decimal>,
    pub ali: Option<Decimal>,
    pub wpa_pos: Option<Decimal>,
    pub wpa_neg: Option<Decimal>,
    pub cwpa: Option<Decimal>,
    pub acli: Option<Decimal>,
    pub re24: Option<Decimal>,
    pub po: Option<i32>,
    pub a: Option<i32>,
    pub details: Option<String>,
    pub counts: DetailCounts,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_detail_tags() {
        let c = DetailCounts::parse(Some("HR,2\u{b7}2B,SB"));
        assert_eq!(c.home_runs, 1);
        assert_eq!(c.doubles, 2);
        assert_eq!(c.stolen_bases, 1);
        assert_eq!(c.triples, 0);
    }

    #[test]
    fn parses_all_tags() {
        let c = DetailCounts::parse(Some("2B,3B,HR,SB,CS,GDP,SF,SH,HBP,IW"));
        assert_eq!(
            (c.doubles, c.triples, c.home_runs, c.stolen_bases, c.caught_stealing),
            (1, 1, 1, 1, 1)
        );
        assert_eq!((c.gdp, c.sac_flies, c.sac_hits, c.hbp, c.ibb), (1, 1, 1, 1, 1));
    }

    #[test]
    fn none_and_empty_are_zero() {
        assert_eq!(DetailCounts::parse(None), DetailCounts::default());
        assert_eq!(DetailCounts::parse(Some("")), DetailCounts::default());
    }
}
