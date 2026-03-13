use chrono::NaiveDate;
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct TournamentRow {
    pub id: i32,
    pub goldfish_id: i32,
    pub name: String,
    pub format: String,
    pub date: NaiveDate,
    pub url: String,
}

#[derive(Debug, FromRow)]
pub struct ArchetypeRow {
    pub id: i32,
    pub name: String,
    pub format: String,
    pub url: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct DeckRow {
    pub id: i32,
    pub goldfish_id: i32,
    pub tournament_id: Option<i32>,
    pub archetype_id: Option<i32>,
    pub player_name: Option<String>,
    pub placement: Option<String>,
    pub format: String,
    pub date: Option<NaiveDate>,
    pub url: String,
}

#[derive(Debug, FromRow)]
pub struct StatsRow {
    pub count: i64,
}
