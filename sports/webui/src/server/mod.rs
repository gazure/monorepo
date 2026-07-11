mod dashboard;
mod games;
mod leaderboards;
mod players;
mod seasons;
mod sql_console;
mod teams;

pub use dashboard::*;
pub use games::*;
pub use leaderboards::*;
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

#[cfg(feature = "server")]
#[expect(clippy::needless_pass_by_value, reason = "used as a map_err callback")]
pub(crate) fn db_err(e: sqlx::Error) -> dioxus::prelude::ServerFnError {
    dioxus::prelude::ServerFnError::new(e.to_string())
}
