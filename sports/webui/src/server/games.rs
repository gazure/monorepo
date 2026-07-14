use dioxus::prelude::*;

use crate::dto::{GameDetailDto, GameSummary, GamesFilter, Page, PlayDto};

/// Shared SELECT + row shape for game summaries, reused by every query that
/// returns games (list, recent, team pages).
#[cfg(feature = "server")]
pub(crate) mod rows {
    use crate::dto::{GameSummary, TeamRef};

    pub const GAME_SUMMARY_SELECT: &str = r"
        SELECT g.id, g.bbref_game_id, g.game_date,
               g.away_score, g.home_score, g.venue, g.attendance,
               ta.id AS away_id, ta.code AS away_code, ta.name AS away_name,
               th.id AS home_id, th.code AS home_code, th.name AS home_name,
               COUNT(*) OVER () AS total
        FROM games g
        JOIN teams ta ON ta.id = g.away_team_id
        JOIN teams th ON th.id = g.home_team_id
    ";

    #[derive(sqlx::FromRow)]
    pub struct GameSummaryRow {
        pub id: i32,
        pub bbref_game_id: String,
        pub game_date: chrono::NaiveDate,
        pub away_score: Option<i32>,
        pub home_score: Option<i32>,
        pub venue: Option<String>,
        pub attendance: Option<i32>,
        pub away_id: i32,
        pub away_code: String,
        pub away_name: String,
        pub home_id: i32,
        pub home_code: String,
        pub home_name: String,
        pub total: i64,
    }

    impl GameSummaryRow {
        pub fn into_dto(self) -> GameSummary {
            GameSummary {
                id: self.id,
                bbref_game_id: self.bbref_game_id,
                game_date: self.game_date,
                away: TeamRef {
                    id: self.away_id,
                    code: self.away_code,
                    name: self.away_name,
                },
                home: TeamRef {
                    id: self.home_id,
                    code: self.home_code,
                    name: self.home_name,
                },
                away_score: self.away_score,
                home_score: self.home_score,
                venue: self.venue,
                attendance: self.attendance,
            }
        }
    }
}

#[server]
pub async fn list_games(filter: GamesFilter, page: u32, page_size: u32) -> Result<Page<GameSummary>, ServerFnError> {
    let page_size = page_size.clamp(1, 200);
    let pool = crate::pool().await?;

    let sql = format!(
        "{select}
         WHERE ($1::date IS NULL OR g.game_date >= $1)
           AND ($2::date IS NULL OR g.game_date <= $2)
           AND ($3::int4 IS NULL OR g.home_team_id = $3 OR g.away_team_id = $3)
           AND ($4::int4 IS NULL OR COALESCE(g.home_score, 0) + COALESCE(g.away_score, 0) >= $4)
           AND ($5::boolean IS NULL OR g.is_night_game = $5)
         ORDER BY g.game_date DESC, g.id DESC
         LIMIT $6 OFFSET $7",
        select = rows::GAME_SUMMARY_SELECT
    );

    // Assembled only from static fragments; user input goes through binds.
    let db_rows: Vec<rows::GameSummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(filter.date_from)
        .bind(filter.date_to)
        .bind(filter.team_id)
        .bind(filter.min_total_runs)
        .bind(filter.night_games)
        .bind(i64::from(page_size))
        .bind(i64::from(page) * i64::from(page_size))
        .fetch_all(pool)
        .await
        .map_err(super::db_err)?;

    let total = db_rows.first().map_or(0, |r| r.total);
    Ok(Page {
        items: db_rows.into_iter().map(rows::GameSummaryRow::into_dto).collect(),
        total,
        page,
        page_size,
    })
}

#[allow(clippy::too_many_lines)]
#[server]
pub async fn game_detail(game_id: i32) -> Result<GameDetailDto, ServerFnError> {
    use crate::dto::{BattingLineDto, LineScore, PitchingLineDto, UmpireDto};

    #[derive(sqlx::FromRow)]
    struct GameRow {
        start_time: Option<String>,
        duration_minutes: Option<i32>,
        weather: Option<String>,
        is_night_game: Option<bool>,
        is_artificial_turf: Option<bool>,
        winning_pitcher: Option<String>,
        losing_pitcher: Option<String>,
        save_pitcher: Option<String>,
    }

    #[derive(sqlx::FromRow)]
    struct LineScoreRow {
        is_home: bool,
        inning: i32,
        runs: i32,
    }

    let pool = crate::pool().await?;

    let sql = format!("{select} WHERE g.id = $1", select = rows::GAME_SUMMARY_SELECT);
    let summary_row: Option<rows::GameSummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(game_id)
        .fetch_optional(pool)
        .await
        .map_err(super::db_err)?;
    let game = summary_row
        .map(rows::GameSummaryRow::into_dto)
        .ok_or_else(|| ServerFnError::new(format!("game {game_id} not found")))?;

    let info: GameRow = sqlx::query_as(
        r"
        SELECT g.start_time, g.duration_minutes, g.weather, g.is_night_game, g.is_artificial_turf,
               wp.name AS winning_pitcher, lp.name AS losing_pitcher, sp.name AS save_pitcher
        FROM games g
        LEFT JOIN players wp ON wp.id = g.winning_pitcher_id
        LEFT JOIN players lp ON lp.id = g.losing_pitcher_id
        LEFT JOIN players sp ON sp.id = g.save_pitcher_id
        WHERE g.id = $1
        ",
    )
    .bind(game_id)
    .fetch_one(pool)
    .await
    .map_err(super::db_err)?;

    let umpires: Vec<UmpireDto> = {
        #[derive(sqlx::FromRow)]
        struct Row {
            position: String,
            name: String,
        }
        sqlx::query_as::<_, Row>("SELECT position, name FROM game_umpires WHERE game_id = $1 ORDER BY id")
            .bind(game_id)
            .fetch_all(pool)
            .await
            .map_err(super::db_err)?
            .into_iter()
            .map(|r| UmpireDto {
                position: r.position,
                name: r.name,
            })
            .collect()
    };

    let line_rows: Vec<LineScoreRow> =
        sqlx::query_as("SELECT is_home, inning, runs FROM game_line_scores WHERE game_id = $1 ORDER BY inning")
            .bind(game_id)
            .fetch_all(pool)
            .await
            .map_err(super::db_err)?;
    let innings = line_rows.iter().map(|r| r.inning).max().unwrap_or(0);
    let innings = usize::try_from(innings).unwrap_or(0);
    let mut line_score = LineScore {
        away: vec![None; innings],
        home: vec![None; innings],
    };
    for row in line_rows {
        if let Ok(idx) = usize::try_from(row.inning - 1) {
            let side = if row.is_home {
                &mut line_score.home
            } else {
                &mut line_score.away
            };
            if let Some(slot) = side.get_mut(idx) {
                *slot = Some(row.runs);
            }
        }
    }

    let batting: Vec<BattingLineDto> = sqlx::query_as::<_, BattingRow>(
        r"
        SELECT bl.player_id, p.name AS player, t.code AS team_code,
               bl.batting_order, bl.position, bl.ab, bl.r, bl.h, bl.rbi, bl.bb, bl.so, bl.pa,
               bl.batting_avg::float8 AS avg, bl.obp::float8 AS obp,
               bl.slg::float8 AS slg, bl.ops::float8 AS ops,
               bl.wpa::float8 AS wpa, bl.re24::float8 AS re24,
               bl.po, bl.a, bl.details
        FROM batting_lines bl
        JOIN players p ON p.id = bl.player_id
        JOIN teams t ON t.id = bl.team_id
        WHERE bl.game_id = $1
        ORDER BY t.code, bl.batting_order ASC NULLS LAST, bl.id
        ",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?
    .into_iter()
    .map(BattingRow::into_dto)
    .collect();

    let pitching: Vec<PitchingLineDto> = sqlx::query_as::<_, PitchingRow>(
        r"
        SELECT pl.player_id, p.name AS player, t.code AS team_code,
               pl.pitch_order, pl.decision, pl.ip::float8 AS ip,
               pl.h, pl.r, pl.er, pl.bb, pl.so, pl.hr,
               pl.era::float8 AS era, pl.batters_faced, pl.pitches, pl.strikes,
               pl.ground_balls, pl.fly_balls, pl.line_drives, pl.game_score,
               pl.wpa::float8 AS wpa
        FROM pitching_lines pl
        JOIN players p ON p.id = pl.player_id
        JOIN teams t ON t.id = pl.team_id
        WHERE pl.game_id = $1
        ORDER BY t.code, pl.pitch_order ASC NULLS LAST, pl.id
        ",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?
    .into_iter()
    .map(PitchingRow::into_dto)
    .collect();

    Ok(GameDetailDto {
        game,
        start_time: info.start_time,
        duration_minutes: info.duration_minutes,
        weather: info.weather,
        is_night_game: info.is_night_game,
        is_artificial_turf: info.is_artificial_turf,
        winning_pitcher: info.winning_pitcher,
        losing_pitcher: info.losing_pitcher,
        save_pitcher: info.save_pitcher,
        umpires,
        line_score,
        batting,
        pitching,
    })
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct BattingRow {
    player_id: i32,
    player: String,
    team_code: String,
    batting_order: Option<i32>,
    position: Option<String>,
    ab: Option<i32>,
    r: Option<i32>,
    h: Option<i32>,
    rbi: Option<i32>,
    bb: Option<i32>,
    so: Option<i32>,
    pa: Option<i32>,
    avg: Option<f64>,
    obp: Option<f64>,
    slg: Option<f64>,
    ops: Option<f64>,
    wpa: Option<f64>,
    re24: Option<f64>,
    po: Option<i32>,
    a: Option<i32>,
    details: Option<String>,
}

#[cfg(feature = "server")]
impl BattingRow {
    fn into_dto(self) -> crate::dto::BattingLineDto {
        crate::dto::BattingLineDto {
            player_id: self.player_id,
            player: self.player,
            team_code: self.team_code,
            batting_order: self.batting_order,
            position: self.position,
            ab: self.ab,
            r: self.r,
            h: self.h,
            rbi: self.rbi,
            bb: self.bb,
            so: self.so,
            pa: self.pa,
            avg: self.avg,
            obp: self.obp,
            slg: self.slg,
            ops: self.ops,
            wpa: self.wpa,
            re24: self.re24,
            po: self.po,
            a: self.a,
            details: self.details,
        }
    }
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
struct PitchingRow {
    player_id: i32,
    player: String,
    team_code: String,
    pitch_order: Option<i32>,
    decision: Option<String>,
    ip: Option<f64>,
    h: Option<i32>,
    r: Option<i32>,
    er: Option<i32>,
    bb: Option<i32>,
    so: Option<i32>,
    hr: Option<i32>,
    era: Option<f64>,
    batters_faced: Option<i32>,
    pitches: Option<i32>,
    strikes: Option<i32>,
    ground_balls: Option<i32>,
    fly_balls: Option<i32>,
    line_drives: Option<i32>,
    game_score: Option<i32>,
    wpa: Option<f64>,
}

#[cfg(feature = "server")]
impl PitchingRow {
    fn into_dto(self) -> crate::dto::PitchingLineDto {
        crate::dto::PitchingLineDto {
            player_id: self.player_id,
            player: self.player,
            team_code: self.team_code,
            pitch_order: self.pitch_order,
            decision: self.decision,
            ip: self.ip,
            h: self.h,
            r: self.r,
            er: self.er,
            bb: self.bb,
            so: self.so,
            hr: self.hr,
            era: self.era,
            batters_faced: self.batters_faced,
            pitches: self.pitches,
            strikes: self.strikes,
            ground_balls: self.ground_balls,
            fly_balls: self.fly_balls,
            line_drives: self.line_drives,
            game_score: self.game_score,
            wpa: self.wpa,
        }
    }
}

#[server]
pub async fn game_play_by_play(game_id: i32) -> Result<Vec<PlayDto>, ServerFnError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        event_num: i32,
        inning: i32,
        is_bottom: bool,
        batting_team: String,
        batter: String,
        pitcher: String,
        outs_before: Option<i32>,
        runners_before: Option<String>,
        score_batting_team: Option<i32>,
        score_fielding_team: Option<i32>,
        pitch_count: Option<i32>,
        runs_on_play: Option<i32>,
        wpa: Option<f64>,
        win_expectancy_after: Option<f64>,
        description: Option<String>,
    }

    let pool = crate::pool().await?;
    let db_rows: Vec<Row> = sqlx::query_as(
        r"
        SELECT pbp.event_num, pbp.inning, pbp.is_bottom, bt.code AS batting_team,
               b.name AS batter, pi.name AS pitcher,
               pbp.outs_before, pbp.runners_before,
               pbp.score_batting_team, pbp.score_fielding_team,
               pbp.pitch_count, pbp.runs_on_play,
               pbp.wpa::float8 AS wpa, pbp.win_expectancy_after::float8 AS win_expectancy_after,
               pbp.play_description AS description
        FROM play_by_play pbp
        JOIN teams bt ON bt.id = pbp.batting_team_id
        JOIN players b ON b.id = pbp.batter_id
        JOIN players pi ON pi.id = pbp.pitcher_id
        WHERE pbp.game_id = $1
        ORDER BY pbp.event_num
        ",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
    .map_err(super::db_err)?;

    Ok(db_rows
        .into_iter()
        .map(|r| PlayDto {
            event_num: r.event_num,
            inning: r.inning,
            is_bottom: r.is_bottom,
            batting_team: r.batting_team,
            batter: r.batter,
            pitcher: r.pitcher,
            outs_before: r.outs_before,
            runners_before: r.runners_before,
            score_batting_team: r.score_batting_team,
            score_fielding_team: r.score_fielding_team,
            pitch_count: r.pitch_count,
            runs_on_play: r.runs_on_play,
            wpa: r.wpa,
            win_expectancy_after: r.win_expectancy_after,
            description: r.description,
        })
        .collect())
}
