use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// One page of results plus the total row count for pagination.
#[expect(clippy::struct_field_names)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

impl<T> Page<T> {
    pub fn total_pages(&self) -> u32 {
        if self.page_size == 0 {
            return 0;
        }
        let size = i64::from(self.page_size);
        u32::try_from((self.total.max(0) + size - 1) / size).unwrap_or(u32::MAX)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamRef {
    pub id: i32,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardStats {
    pub teams: i64,
    pub players: i64,
    pub games: i64,
    pub batting_lines: i64,
    pub pitching_lines: i64,
    pub plays: i64,
    pub first_game: Option<NaiveDate>,
    pub last_game: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeasonGamesCount {
    pub season: i32,
    pub games: i64,
}

/// A recent game ranked by total win-probability movement, with its
/// home-perspective win-expectancy arc for a sparkline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DramaticGame {
    pub game: GameSummary,
    /// Sum of absolute per-play WPA, in percentage points
    pub swing: f64,
    pub we_home: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameSummary {
    pub id: i32,
    pub bbref_game_id: String,
    pub game_date: NaiveDate,
    pub away: TeamRef,
    pub home: TeamRef,
    pub away_score: Option<i32>,
    pub home_score: Option<i32>,
    pub venue: Option<String>,
    pub attendance: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GamesFilter {
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub team_id: Option<i32>,
    pub min_total_runs: Option<i32>,
    pub night_games: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UmpireDto {
    pub position: String,
    pub name: String,
}

/// Runs per inning for both teams, padded to the same length.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LineScore {
    pub away: Vec<Option<i32>>,
    pub home: Vec<Option<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BattingLineDto {
    pub player_id: i32,
    pub player: String,
    pub team_code: String,
    pub batting_order: Option<i32>,
    pub position: Option<String>,
    pub ab: Option<i32>,
    pub r: Option<i32>,
    pub h: Option<i32>,
    pub rbi: Option<i32>,
    pub bb: Option<i32>,
    pub so: Option<i32>,
    pub pa: Option<i32>,
    pub avg: Option<f64>,
    pub obp: Option<f64>,
    pub slg: Option<f64>,
    pub ops: Option<f64>,
    pub wpa: Option<f64>,
    pub re24: Option<f64>,
    pub po: Option<i32>,
    pub a: Option<i32>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitchingLineDto {
    pub player_id: i32,
    pub player: String,
    pub team_code: String,
    pub pitch_order: Option<i32>,
    pub decision: Option<String>,
    pub ip: Option<f64>,
    pub h: Option<i32>,
    pub r: Option<i32>,
    pub er: Option<i32>,
    pub bb: Option<i32>,
    pub so: Option<i32>,
    pub hr: Option<i32>,
    pub era: Option<f64>,
    pub batters_faced: Option<i32>,
    pub pitches: Option<i32>,
    pub strikes: Option<i32>,
    pub ground_balls: Option<i32>,
    pub fly_balls: Option<i32>,
    pub line_drives: Option<i32>,
    pub game_score: Option<i32>,
    pub wpa: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameDetailDto {
    pub game: GameSummary,
    pub start_time: Option<String>,
    pub duration_minutes: Option<i32>,
    pub weather: Option<String>,
    pub is_night_game: Option<bool>,
    pub is_artificial_turf: Option<bool>,
    pub winning_pitcher: Option<String>,
    pub losing_pitcher: Option<String>,
    pub save_pitcher: Option<String>,
    pub umpires: Vec<UmpireDto>,
    pub line_score: LineScore,
    pub batting: Vec<BattingLineDto>,
    pub pitching: Vec<PitchingLineDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayDto {
    pub event_num: i32,
    pub inning: i32,
    pub is_bottom: bool,
    pub batting_team: String,
    pub batter: String,
    pub pitcher: String,
    pub outs_before: Option<i32>,
    pub runners_before: Option<String>,
    pub score_batting_team: Option<i32>,
    pub score_fielding_team: Option<i32>,
    pub pitch_count: Option<i32>,
    pub pitch_sequence: Option<String>,
    pub runs_on_play: Option<i32>,
    pub wpa: Option<f64>,
    pub win_expectancy_after: Option<f64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerHit {
    pub id: i32,
    pub bbref_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BattingTotals {
    pub games: i64,
    pub pa: i64,
    pub ab: i64,
    pub h: i64,
    pub r: i64,
    pub rbi: i64,
    pub bb: i64,
    pub so: i64,
    pub doubles: i64,
    pub triples: i64,
    pub home_runs: i64,
    pub stolen_bases: i64,
    pub avg: Option<f64>,
    pub obp: Option<f64>,
    pub slg: Option<f64>,
    pub ops: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitchingTotals {
    pub games: i64,
    pub outs: i64,
    pub h: i64,
    pub r: i64,
    pub er: i64,
    pub bb: i64,
    pub so: i64,
    pub hr: i64,
    pub era: Option<f64>,
    pub whip: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BattingSeasonRow {
    pub season: i32,
    pub postseason: bool,
    pub games: i64,
    pub pa: i64,
    pub ab: i64,
    pub h: i64,
    pub r: i64,
    pub rbi: i64,
    pub bb: i64,
    pub so: i64,
    pub doubles: i64,
    pub triples: i64,
    pub home_runs: i64,
    pub stolen_bases: i64,
    pub avg: Option<f64>,
    pub obp: Option<f64>,
    pub slg: Option<f64>,
    pub ops: Option<f64>,
    pub wpa: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitchingSeasonRow {
    pub season: i32,
    pub postseason: bool,
    pub games: i64,
    pub outs: i64,
    pub h: i64,
    pub r: i64,
    pub er: i64,
    pub bb: i64,
    pub so: i64,
    pub hr: i64,
    pub wins: i64,
    pub losses: i64,
    pub saves: i64,
    pub era: Option<f64>,
    pub whip: Option<f64>,
    pub wpa: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerDetailDto {
    pub player: PlayerHit,
    pub batting: Option<BattingTotals>,
    pub pitching: Option<PitchingTotals>,
    pub batting_postseason: Option<BattingTotals>,
    pub pitching_postseason: Option<PitchingTotals>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BattingGameLogRow {
    pub game_id: i32,
    pub game_date: NaiveDate,
    pub team_code: String,
    pub opponent_code: String,
    pub position: Option<String>,
    pub ab: Option<i32>,
    pub r: Option<i32>,
    pub h: Option<i32>,
    pub rbi: Option<i32>,
    pub bb: Option<i32>,
    pub so: Option<i32>,
    pub pa: Option<i32>,
    pub wpa: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitchingGameLogRow {
    pub game_id: i32,
    pub game_date: NaiveDate,
    pub team_code: String,
    pub opponent_code: String,
    pub decision: Option<String>,
    pub ip: Option<f64>,
    pub h: Option<i32>,
    pub r: Option<i32>,
    pub er: Option<i32>,
    pub bb: Option<i32>,
    pub so: Option<i32>,
    pub hr: Option<i32>,
    pub pitches: Option<i32>,
    pub game_score: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamSummary {
    pub team: TeamRef,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub runs_for: i64,
    pub runs_against: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamDetailDto {
    pub summary: TeamSummary,
    pub recent_games: Vec<GameSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchupEvent {
    pub game_id: i32,
    pub game_date: NaiveDate,
    pub inning: i32,
    pub is_bottom: bool,
    pub description: Option<String>,
    pub wpa: Option<f64>,
}

/// Head-to-head plate-appearance tally, classified from play descriptions
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MatchupTally {
    pub pa: i64,
    pub singles: i64,
    pub doubles: i64,
    pub triples: i64,
    pub home_runs: i64,
    pub walks: i64,
    pub hbp: i64,
    pub strikeouts: i64,
    pub other_outs: i64,
}

impl MatchupTally {
    pub fn hits(&self) -> i64 {
        self.singles + self.doubles + self.triples + self.home_runs
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchupDto {
    pub batter: String,
    pub pitcher: String,
    pub tally: MatchupTally,
    pub events: Vec<MatchupEvent>,
}

/// One split line (home/road or vs one opponent)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SplitRow {
    pub label: String,
    pub games: i64,
    pub pa: i64,
    pub h: i64,
    pub home_runs: i64,
    pub avg: Option<f64>,
    pub obp: Option<f64>,
    pub slg: Option<f64>,
    pub ops: Option<f64>,
}

/// One pitching split line (home/road or vs one opponent)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitcherSplitRow {
    pub label: String,
    pub games: i64,
    pub outs: i64,
    pub so: i64,
    pub bb: i64,
    pub era: Option<f64>,
    pub whip: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitcherSplitsDto {
    pub home_away: Vec<PitcherSplitRow>,
    pub vs_team: Vec<PitcherSplitRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerSplitsDto {
    pub home_away: Vec<SplitRow>,
    pub vs_team: Vec<SplitRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerBrowseRow {
    pub player_id: i32,
    pub name: String,
    pub games: i64,
    pub pa: i64,
    pub h: i64,
    pub home_runs: i64,
    pub stolen_bases: i64,
    pub avg: Option<f64>,
    pub obp: Option<f64>,
    pub slg: Option<f64>,
    pub ops: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerBrowseSort {
    #[default]
    Pa,
    Hits,
    HomeRuns,
    StolenBases,
    Ops,
}

impl PlayerBrowseSort {
    pub const ALL: [Self; 5] = [Self::Pa, Self::Hits, Self::HomeRuns, Self::StolenBases, Self::Ops];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pa => "Career PA",
            Self::Hits => "Career H",
            Self::HomeRuns => "Career HR",
            Self::StolenBases => "Career SB",
            Self::Ops => "Career OPS",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeadToHeadRow {
    pub opponent: TeamRef,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamSeasonRow {
    pub season: i32,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub runs_for: i64,
    pub runs_against: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RosterBatter {
    pub player_id: i32,
    pub name: String,
    pub games: i64,
    pub pa: i64,
    pub h: i64,
    pub home_runs: i64,
    pub stolen_bases: i64,
    pub avg: Option<f64>,
    pub obp: Option<f64>,
    pub slg: Option<f64>,
    pub ops: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RosterPitcher {
    pub player_id: i32,
    pub name: String,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub saves: i64,
    pub outs: i64,
    pub so: i64,
    pub era: Option<f64>,
    pub whip: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamRosterDto {
    pub batters: Vec<RosterBatter>,
    pub pitchers: Vec<RosterPitcher>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum BattingSort {
    #[default]
    Ops,
    Avg,
    Obp,
    Slg,
    HomeRuns,
    Doubles,
    Triples,
    StolenBases,
    Hits,
    Runs,
    Rbi,
    Walks,
    Strikeouts,
    Pa,
    Wpa,
}

impl BattingSort {
    pub const ALL: [Self; 15] = [
        Self::Ops,
        Self::Avg,
        Self::Obp,
        Self::Slg,
        Self::HomeRuns,
        Self::Doubles,
        Self::Triples,
        Self::StolenBases,
        Self::Hits,
        Self::Runs,
        Self::Rbi,
        Self::Walks,
        Self::Strikeouts,
        Self::Pa,
        Self::Wpa,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ops => "OPS",
            Self::Avg => "AVG",
            Self::Obp => "OBP",
            Self::Slg => "SLG",
            Self::HomeRuns => "HR",
            Self::Doubles => "2B",
            Self::Triples => "3B",
            Self::StolenBases => "SB",
            Self::Hits => "H",
            Self::Runs => "R",
            Self::Rbi => "RBI",
            Self::Walks => "BB",
            Self::Strikeouts => "SO",
            Self::Pa => "PA",
            Self::Wpa => "WPA",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum PitchingSort {
    #[default]
    Era,
    Whip,
    Strikeouts,
    InningsPitched,
    Wins,
    Saves,
    Walks,
    HomeRuns,
    Wpa,
}

impl PitchingSort {
    pub const ALL: [Self; 9] = [
        Self::Era,
        Self::Whip,
        Self::Strikeouts,
        Self::InningsPitched,
        Self::Wins,
        Self::Saves,
        Self::Walks,
        Self::HomeRuns,
        Self::Wpa,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Era => "ERA",
            Self::Whip => "WHIP",
            Self::Strikeouts => "SO",
            Self::InningsPitched => "IP",
            Self::Wins => "W",
            Self::Saves => "SV",
            Self::Walks => "BB",
            Self::HomeRuns => "HR",
            Self::Wpa => "WPA",
        }
    }
}

/// One postseason series, winner first
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BracketSeries {
    pub winner: TeamRef,
    pub winner_wins: i64,
    pub loser: TeamRef,
    pub loser_wins: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SeasonSummary {
    pub season: i32,
    pub games: i64,
    pub teams: i64,
    pub runs: i64,
    pub runs_per_game: Option<f64>,
    pub attendance: i64,
    pub avg_attendance: Option<f64>,
    pub postseason_games: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BattingLeaderboardReq {
    pub sort: BattingSort,
    pub min_pa: i64,
    pub season: Option<i32>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BattingLeaderRow {
    pub player_id: i32,
    pub name: String,
    pub games: i64,
    pub pa: i64,
    pub ab: i64,
    pub h: i64,
    pub r: i64,
    pub rbi: i64,
    pub bb: i64,
    pub so: i64,
    pub doubles: i64,
    pub triples: i64,
    pub home_runs: i64,
    pub stolen_bases: i64,
    pub avg: Option<f64>,
    pub obp: Option<f64>,
    pub slg: Option<f64>,
    pub ops: Option<f64>,
    pub wpa: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitchingLeaderboardReq {
    pub sort: PitchingSort,
    pub min_outs: i64,
    pub season: Option<i32>,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PitchingLeaderRow {
    pub player_id: i32,
    pub name: String,
    pub games: i64,
    pub outs: i64,
    pub h: i64,
    pub r: i64,
    pub er: i64,
    pub bb: i64,
    pub so: i64,
    pub hr: i64,
    pub wins: i64,
    pub losses: i64,
    pub saves: i64,
    pub era: Option<f64>,
    pub whip: Option<f64>,
    pub wpa: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub row_count: usize,
    pub truncated: bool,
    pub elapsed_ms: u64,
}

/// Format a total expressed in outs as baseball innings-pitched notation (e.g. 62 outs -> "20.2").
pub fn format_ip(outs: i64) -> String {
    format!("{}.{}", outs / 3, outs % 3)
}
