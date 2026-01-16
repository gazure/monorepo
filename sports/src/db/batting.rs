use sqlx::PgPool;

use crate::models::NewBattingLine;

/// Insert batting lines for a game
pub async fn insert_batting_lines(pool: &PgPool, lines: &[NewBattingLine]) -> Result<(), sqlx::Error> {
    for line in lines {
        sqlx::query(
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
        )
        .bind(line.game_id)
        .bind(line.player_id)
        .bind(line.team_id)
        .bind(line.batting_order)
        .bind(&line.position)
        .bind(line.ab)
        .bind(line.r)
        .bind(line.h)
        .bind(line.rbi)
        .bind(line.bb)
        .bind(line.so)
        .bind(line.pa)
        .bind(line.batting_avg)
        .bind(line.obp)
        .bind(line.slg)
        .bind(line.ops)
        .bind(line.pitches_seen)
        .bind(line.strikes_seen)
        .bind(line.wpa)
        .bind(line.ali)
        .bind(line.wpa_pos)
        .bind(line.wpa_neg)
        .bind(line.cwpa)
        .bind(line.acli)
        .bind(line.re24)
        .bind(line.po)
        .bind(line.a)
        .bind(&line.details)
        .execute(pool)
        .await?;
    }

    Ok(())
}
