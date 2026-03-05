use sqlx::PgPool;

use crate::models::{Game, NewGame, NewGameLineScore, NewGameUmpire};

/// Insert a new game, returning the game with its ID
pub async fn insert_game(pool: &PgPool, game: &NewGame) -> Result<Game, sqlx::Error> {
    sqlx::query_as!(
        Game,
        r"
        INSERT INTO games (
            bbref_game_id, game_date, start_time, venue, attendance,
            duration_minutes, weather, is_night_game, is_artificial_turf,
            home_team_id, away_team_id, home_score, away_score,
            winning_pitcher_id, losing_pitcher_id, save_pitcher_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        RETURNING id, bbref_game_id, game_date, start_time, venue, attendance,
            duration_minutes, weather, is_night_game, is_artificial_turf,
            home_team_id, away_team_id, home_score, away_score,
            winning_pitcher_id, losing_pitcher_id, save_pitcher_id, created_at
        ",
        game.bbref_game_id,
        game.game_date,
        game.start_time,
        game.venue,
        game.attendance,
        game.duration_minutes,
        game.weather,
        game.is_night_game,
        game.is_artificial_turf,
        game.home_team_id,
        game.away_team_id,
        game.home_score,
        game.away_score,
        game.winning_pitcher_id,
        game.losing_pitcher_id,
        game.save_pitcher_id,
    )
    .fetch_one(pool)
    .await
}

/// Check if a game already exists by `bbref_game_id`
pub async fn game_exists(pool: &PgPool, bbref_game_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r"
        SELECT COUNT(*) FROM games WHERE bbref_game_id = $1
        ",
        bbref_game_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(result.unwrap_or(0) > 0)
}

/// Insert umpires for a game
pub async fn insert_umpires(pool: &PgPool, umpires: &[NewGameUmpire]) -> Result<(), sqlx::Error> {
    for umpire in umpires {
        sqlx::query!(
            r"
            INSERT INTO game_umpires (game_id, position, name)
            VALUES ($1, $2, $3)
            ",
            umpire.game_id,
            umpire.position,
            umpire.name,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Insert line scores for a game
pub async fn insert_line_scores(pool: &PgPool, line_scores: &[NewGameLineScore]) -> Result<(), sqlx::Error> {
    for ls in line_scores {
        sqlx::query!(
            r"
            INSERT INTO game_line_scores (game_id, team_id, is_home, inning, runs)
            VALUES ($1, $2, $3, $4, $5)
            ",
            ls.game_id,
            ls.team_id,
            ls.is_home,
            ls.inning,
            ls.runs,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
