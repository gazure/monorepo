-- Restore named pools, model relationships explicitly, and make draws
-- reproducible and non-destructive.

-- ---------------------------------------------------------------------------
-- Pools
-- ---------------------------------------------------------------------------
CREATE TABLE pool (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    slug VARCHAR(255) NOT NULL UNIQUE CHECK (slug ~ '^[a-z0-9-]+$'),
    description TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE pool_membership (
    pool_id INTEGER NOT NULL REFERENCES pool(id) ON DELETE CASCADE,
    participant_id INTEGER NOT NULL REFERENCES participant(id) ON DELETE CASCADE,
    PRIMARY KEY (pool_id, participant_id)
);

CREATE INDEX idx_pool_membership_participant ON pool_membership(participant_id);

-- ---------------------------------------------------------------------------
-- Typed relationships
--
-- Previously a spouse was just an `exclusion` row with a free-text `reason`,
-- which gave the "no spouses" toggle nothing to key off.
-- ---------------------------------------------------------------------------
CREATE TYPE relationship_kind AS ENUM ('spouse', 'household', 'manual');

ALTER TABLE exclusion ADD COLUMN kind relationship_kind NOT NULL DEFAULT 'manual';

-- ---------------------------------------------------------------------------
-- Letters become per-pool
-- ---------------------------------------------------------------------------
ALTER TABLE excluded_letter RENAME TO excluded_letter_old;

CREATE TABLE excluded_letter (
    pool_id INTEGER NOT NULL REFERENCES pool(id) ON DELETE CASCADE,
    letter CHAR(1) NOT NULL CHECK (letter ~ '^[A-Z]$'),
    PRIMARY KEY (pool_id, letter)
);

-- ---------------------------------------------------------------------------
-- Exchanges become pool-scoped, reproducible, and revisioned
-- ---------------------------------------------------------------------------
ALTER TABLE exchange ADD COLUMN pool_id INTEGER REFERENCES pool(id) ON DELETE CASCADE;
-- Draw settings snapshotted alongside the result so old years stay explainable.
ALTER TABLE exchange ADD COLUMN config JSONB NOT NULL DEFAULT '{}'::jsonb;
-- RNG seed, so any draw can be replayed and audited.
ALTER TABLE exchange ADD COLUMN seed BIGINT;
-- Re-drawing inserts a new revision instead of destroying the previous result.
ALTER TABLE exchange ADD COLUMN revision INTEGER NOT NULL DEFAULT 1;

-- ---------------------------------------------------------------------------
-- Backfill: existing rows predate pools, so give them a default pool.
-- Skipped entirely on a fresh database so no stray pool is created.
-- ---------------------------------------------------------------------------
DO $$
DECLARE
    default_pool_id INTEGER;
BEGIN
    IF EXISTS (SELECT 1 FROM participant) OR EXISTS (SELECT 1 FROM exchange) THEN
        INSERT INTO pool (name, slug, description, sort_order)
        VALUES ('Family', 'family', 'Migrated from the pre-pool schema', 0)
        RETURNING id INTO default_pool_id;

        INSERT INTO pool_membership (pool_id, participant_id)
        SELECT default_pool_id, id FROM participant;

        INSERT INTO excluded_letter (pool_id, letter)
        SELECT default_pool_id, letter FROM excluded_letter_old;

        UPDATE exchange SET pool_id = default_pool_id;
    END IF;
END $$;

DROP TABLE excluded_letter_old;

ALTER TABLE exchange ALTER COLUMN pool_id SET NOT NULL;

-- One draw per year gave way to one draw per pool per year, with history.
ALTER TABLE exchange DROP CONSTRAINT IF EXISTS exchange_year_key;
ALTER TABLE exchange ADD CONSTRAINT exchange_pool_year_revision_key UNIQUE (pool_id, year, revision);

CREATE INDEX idx_exchange_pool_year ON exchange(pool_id, year DESC, revision DESC);
