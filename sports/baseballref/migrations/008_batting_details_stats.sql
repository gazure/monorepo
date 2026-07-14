-- Counting stats parsed from the box-score `details` tags (2B, 3B, HR, SB,
-- CS, GDP, SF, SH, HBP, IW). Absent tag = zero, so NOT NULL DEFAULT 0 is
-- semantically correct. Existing rows are backfilled from `details` by
-- scripts/backfill_details_stats.sql.
ALTER TABLE batting_lines
    ADD COLUMN doubles INT NOT NULL DEFAULT 0,
    ADD COLUMN triples INT NOT NULL DEFAULT 0,
    ADD COLUMN home_runs INT NOT NULL DEFAULT 0,
    ADD COLUMN stolen_bases INT NOT NULL DEFAULT 0,
    ADD COLUMN caught_stealing INT NOT NULL DEFAULT 0,
    ADD COLUMN gdp INT NOT NULL DEFAULT 0,
    ADD COLUMN sac_flies INT NOT NULL DEFAULT 0,
    ADD COLUMN sac_hits INT NOT NULL DEFAULT 0,
    ADD COLUMN hbp INT NOT NULL DEFAULT 0,
    ADD COLUMN ibb INT NOT NULL DEFAULT 0;
