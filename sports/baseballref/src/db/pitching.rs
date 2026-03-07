use sqlx::PgPool;

use crate::models::NewPitchingLine;

/// Insert pitching lines for a game
pub async fn insert_pitching_lines(pool: &PgPool, lines: &[NewPitchingLine]) -> Result<(), sqlx::Error> {
    for line in lines {
        sqlx::query!(
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
            line.game_id,
            line.player_id,
            line.team_id,
            line.pitch_order,
            line.decision,
            line.ip,
            line.h,
            line.r,
            line.er,
            line.bb,
            line.so,
            line.hr,
            line.era,
            line.batters_faced,
            line.pitches,
            line.strikes,
            line.strikes_contact,
            line.strikes_swinging,
            line.strikes_looking,
            line.ground_balls,
            line.fly_balls,
            line.line_drives,
            line.game_score,
            line.inherited_runners,
            line.inherited_scored,
            line.wpa,
            line.ali,
            line.cwpa,
            line.acli,
            line.re24,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
