use sqlx::PgPool;

use crate::models::NewPlayByPlay;

/// Insert play-by-play events for a game
pub async fn insert_play_by_play(pool: &PgPool, events: &[NewPlayByPlay]) -> Result<(), sqlx::Error> {
    for event in events {
        sqlx::query(
            r"
            INSERT INTO play_by_play (
                game_id, event_num, inning, is_bottom,
                batting_team_id, batter_id, pitcher_id,
                outs_before, runners_before,
                score_batting_team, score_fielding_team,
                pitch_sequence, pitch_count,
                runs_on_play, outs_on_play,
                wpa, win_expectancy_after, play_description
            )
            VALUES (
                $1, $2, $3, $4,
                $5, $6, $7,
                $8, $9,
                $10, $11,
                $12, $13,
                $14, $15,
                $16, $17, $18
            )
            ",
        )
        .bind(event.game_id)
        .bind(event.event_num)
        .bind(event.inning)
        .bind(event.is_bottom)
        .bind(event.batting_team_id)
        .bind(event.batter_id)
        .bind(event.pitcher_id)
        .bind(event.outs_before)
        .bind(&event.runners_before)
        .bind(event.score_batting_team)
        .bind(event.score_fielding_team)
        .bind(&event.pitch_sequence)
        .bind(event.pitch_count)
        .bind(event.runs_on_play)
        .bind(event.outs_on_play)
        .bind(event.wpa)
        .bind(event.win_expectancy_after)
        .bind(&event.play_description)
        .execute(pool)
        .await?;
    }

    Ok(())
}
