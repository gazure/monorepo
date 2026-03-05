use sqlx::PgPool;

use crate::models::NewPlayByPlay;

/// Insert play-by-play events for a game
pub async fn insert_play_by_play(pool: &PgPool, events: &[NewPlayByPlay]) -> Result<(), sqlx::Error> {
    for event in events {
        sqlx::query!(
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
            event.game_id,
            event.event_num,
            event.inning,
            event.is_bottom,
            event.batting_team_id,
            event.batter_id,
            event.pitcher_id,
            event.outs_before,
            event.runners_before,
            event.score_batting_team,
            event.score_fielding_team,
            event.pitch_sequence,
            event.pitch_count,
            event.runs_on_play,
            event.outs_on_play,
            event.wpa,
            event.win_expectancy_after,
            event.play_description,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
