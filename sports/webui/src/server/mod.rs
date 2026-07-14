mod dashboard;
mod games;
mod leaderboards;
mod matchups;
mod players;
mod seasons;
mod sql_console;
mod teams;

pub use dashboard::*;
pub use games::*;
pub use leaderboards::*;
pub(crate) use matchups::is_baserunning_only;
pub use matchups::*;
pub use players::*;
pub use seasons::*;
pub use sql_console::*;
pub use teams::*;

/// CTE body computing each season's last regular-season date: the schema has
/// no postseason flag, so a game is treated as postseason when it falls after
/// its year's last date with 6+ games league-wide (postseason days never
/// exceed 4). Known limitation: game-163 tiebreakers classify as postseason.
#[cfg(feature = "server")]
pub(crate) const REGULAR_SEASON_END: &str = r"
    SELECT season, MAX(game_date) AS end_date
    FROM (
        SELECT EXTRACT(YEAR FROM game_date)::int4 AS season, game_date, COUNT(*) AS n
        FROM games
        GROUP BY 1, 2
    ) date_counts
    WHERE n >= 6
    GROUP BY season
";

/// Exact aggregate batting rate stats over `batting_lines bl`, using the
/// details-derived counting columns: TB = H + 2B + 2·3B + 3·HR,
/// OBP = (H+BB+HBP)/(AB+BB+HBP+SF). Emits `avg`, `obp`, `slg` columns.
#[cfg(feature = "server")]
pub(crate) const BATTING_RATE_SQL: &str = r"
    SUM(bl.h)::float8 / NULLIF(SUM(bl.ab), 0)::float8 AS avg,
    (SUM(bl.h) + SUM(bl.bb) + SUM(bl.hbp))::float8
        / NULLIF(SUM(bl.ab) + SUM(bl.bb) + SUM(bl.hbp) + SUM(bl.sac_flies), 0)::float8 AS obp,
    (SUM(bl.h) + SUM(bl.doubles) + 2 * SUM(bl.triples) + 3 * SUM(bl.home_runs))::float8
        / NULLIF(SUM(bl.ab), 0)::float8 AS slg
";

/// Details-derived counting sums over `batting_lines bl`
#[cfg(feature = "server")]
pub(crate) const BATTING_COUNT_SQL: &str = r"
    COALESCE(SUM(bl.doubles), 0)::bigint AS doubles,
    COALESCE(SUM(bl.triples), 0)::bigint AS triples,
    COALESCE(SUM(bl.home_runs), 0)::bigint AS home_runs,
    COALESCE(SUM(bl.stolen_bases), 0)::bigint AS stolen_bases
";

#[cfg(feature = "server")]
#[expect(clippy::needless_pass_by_value, reason = "used as a map_err callback")]
pub(crate) fn db_err(e: sqlx::Error) -> dioxus::prelude::ServerFnError {
    dioxus::prelude::ServerFnError::new(e.to_string())
}
