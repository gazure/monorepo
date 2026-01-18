use sqlx::PgPool;

use crate::models::NewPitchingLine;

/// Insert pitching lines for a game
pub async fn insert_pitching_lines(pool: &PgPool, lines: &[NewPitchingLine]) -> Result<(), sqlx::Error> {
    for line in lines {
        sqlx::query(
            r"
            INSERT INTO pitching_lines (
                game_id, player_id, team_id, pitch_order, decision,
                ip, h, r, er, bb, so, hr, era,
                batters_faced, pitches, strikes,
                strikes_contact, strikes_swinging, strikes_looking,
                ground_balls, fly_balls, line_drives,
                game_score, inherited_runners, inherited_scored,
                wpa, ali, cwpa, acli, re24
            )
            VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10, $11, $12, $13,
                $14, $15, $16,
                $17, $18, $19,
                $20, $21, $22,
                $23, $24, $25,
                $26, $27, $28, $29, $30
            )
            ",
        )
        .bind(line.game_id)
        .bind(line.player_id)
        .bind(line.team_id)
        .bind(line.pitch_order)
        .bind(&line.decision)
        .bind(line.ip)
        .bind(line.h)
        .bind(line.r)
        .bind(line.er)
        .bind(line.bb)
        .bind(line.so)
        .bind(line.hr)
        .bind(line.era)
        .bind(line.batters_faced)
        .bind(line.pitches)
        .bind(line.strikes)
        .bind(line.strikes_contact)
        .bind(line.strikes_swinging)
        .bind(line.strikes_looking)
        .bind(line.ground_balls)
        .bind(line.fly_balls)
        .bind(line.line_drives)
        .bind(line.game_score)
        .bind(line.inherited_runners)
        .bind(line.inherited_scored)
        .bind(line.wpa)
        .bind(line.ali)
        .bind(line.cwpa)
        .bind(line.acli)
        .bind(line.re24)
        .execute(pool)
        .await?;
    }

    Ok(())
}
