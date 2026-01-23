use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

/// A record of a failed scrape attempt
#[derive(Debug, Clone, FromRow)]
pub struct FailedScrape {
    pub id: i32,
    pub bbref_game_id: String,
    pub error_message: String,
    pub failed_at: DateTime<Utc>,
    pub attempt_count: i32,
}

/// Database operations for failed scrapes
pub struct FailedScrapesDb<'a> {
    pool: &'a PgPool,
}

impl<'a> FailedScrapesDb<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Record a failure, incrementing attempt count if it already exists
    pub async fn record_failure(&self, game_id: &str, error: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO failed_scrapes (bbref_game_id, error_message, failed_at, attempt_count)
            VALUES ($1, $2, NOW(), 1)
            ON CONFLICT (bbref_game_id)
            DO UPDATE SET
                error_message = EXCLUDED.error_message,
                failed_at = NOW(),
                attempt_count = failed_scrapes.attempt_count + 1
            ",
        )
        .bind(game_id)
        .bind(error)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// List all failed scrapes, ordered by most recent first
    pub async fn list_failures(&self) -> Result<Vec<FailedScrape>, sqlx::Error> {
        let rows = sqlx::query_as::<_, FailedScrape>(
            r"
            SELECT id, bbref_game_id, error_message, failed_at, attempt_count
            FROM failed_scrapes
            ORDER BY failed_at DESC
            ",
        )
        .fetch_all(self.pool)
        .await?;

        Ok(rows)
    }

    /// Delete a single failure by game ID
    pub async fn delete_failure(&self, game_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM failed_scrapes WHERE bbref_game_id = $1")
            .bind(game_id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Clear all failures
    pub async fn clear_failures(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM failed_scrapes").execute(self.pool).await?;

        Ok(result.rows_affected())
    }

    /// Get a single failure by game ID
    pub async fn get_failure(&self, game_id: &str) -> Result<Option<FailedScrape>, sqlx::Error> {
        let row = sqlx::query_as::<_, FailedScrape>(
            r"
            SELECT id, bbref_game_id, error_message, failed_at, attempt_count
            FROM failed_scrapes
            WHERE bbref_game_id = $1
            ",
        )
        .bind(game_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row)
    }

    /// Get count of failed scrapes
    pub async fn count_failures(&self) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM failed_scrapes")
            .fetch_one(self.pool)
            .await?;

        Ok(row.0)
    }
}
