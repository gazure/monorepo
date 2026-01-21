-- Failed scrapes tracking table
-- Stores game IDs that failed to scrape/import for later retry

CREATE TABLE failed_scrapes (
    id SERIAL PRIMARY KEY,
    bbref_game_id VARCHAR(20) UNIQUE NOT NULL,
    error_message TEXT NOT NULL,
    failed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    attempt_count INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_failed_scrapes_game_id ON failed_scrapes(bbref_game_id);
CREATE INDEX idx_failed_scrapes_failed_at ON failed_scrapes(failed_at);
