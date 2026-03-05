use sqlx::PgPool;

use crate::models::NewBattingLine;

/// Insert batting lines for a game
pub async fn insert_batting_lines(pool: &PgPool, lines: &[NewBattingLine]) -> Result<(), sqlx::Error> {
    for line in lines {
        sqlx::query!(
            r"
            INSERT INTO batting_lines (
                game_id, player_id, team_id, batting_order, position,
                ab, r, h, rbi, bb, so, pa,
                batting_avg, obp, slg, ops,
                pitches_seen, strikes_seen,
                wpa, ali, wpa_pos, wpa_neg, cwpa, acli, re24,
                po, a, details
            )
            VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16,
                $17, $18,
                $19, $20, $21, $22, $23, $24, $25,
                $26, $27, $28
            )
            ",
            line.game_id,
            line.player_id,
            line.team_id,
            line.batting_order,
            line.position,
            line.ab,
            line.r,
            line.h,
            line.rbi,
            line.bb,
            line.so,
            line.pa,
            line.batting_avg,
            line.obp,
            line.slg,
            line.ops,
            line.pitches_seen,
            line.strikes_seen,
            line.wpa,
            line.ali,
            line.wpa_pos,
            line.wpa_neg,
            line.cwpa,
            line.acli,
            line.re24,
            line.po,
            line.a,
            line.details,
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
