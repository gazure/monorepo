-- Backfill the counting columns added in migration 008 from the box-score
-- `details` tag string. Tags are comma-separated with an `N·TAG` multiplier
-- form ('·' is U+00B7); the vocabulary is closed: 2B, 3B, HR, SB, CS, GDP,
-- SF, SH, HBP, IW.
--
-- Idempotent and re-runnable (e.g. after new games are scraped by a binary
-- that predates the parser forward-fill).
--
-- Run: psql "$SPORTS_DATABASE_URL" -f backfill_details_stats.sql

BEGIN;

WITH parsed AS (
    SELECT id,
           COALESCE(SUM(n) FILTER (WHERE tag = '2B'), 0)::int  AS doubles,
           COALESCE(SUM(n) FILTER (WHERE tag = '3B'), 0)::int  AS triples,
           COALESCE(SUM(n) FILTER (WHERE tag = 'HR'), 0)::int  AS home_runs,
           COALESCE(SUM(n) FILTER (WHERE tag = 'SB'), 0)::int  AS stolen_bases,
           COALESCE(SUM(n) FILTER (WHERE tag = 'CS'), 0)::int  AS caught_stealing,
           COALESCE(SUM(n) FILTER (WHERE tag = 'GDP'), 0)::int AS gdp,
           COALESCE(SUM(n) FILTER (WHERE tag = 'SF'), 0)::int  AS sac_flies,
           COALESCE(SUM(n) FILTER (WHERE tag = 'SH'), 0)::int  AS sac_hits,
           COALESCE(SUM(n) FILTER (WHERE tag = 'HBP'), 0)::int AS hbp,
           COALESCE(SUM(n) FILTER (WHERE tag = 'IW'), 0)::int  AS ibb
    FROM (
        SELECT bl.id,
               CASE WHEN item LIKE '%·%' THEN split_part(item, '·', 1)::int ELSE 1 END AS n,
               CASE WHEN item LIKE '%·%' THEN split_part(item, '·', 2) ELSE item END   AS tag
        FROM batting_lines bl,
             LATERAL unnest(string_to_array(bl.details, ',')) AS raw(item0),
             LATERAL (SELECT trim(raw.item0) AS item) AS t
        WHERE bl.details IS NOT NULL AND bl.details <> ''
    ) tags
    GROUP BY id
)
UPDATE batting_lines bl
SET doubles        = p.doubles,
    triples        = p.triples,
    home_runs      = p.home_runs,
    stolen_bases   = p.stolen_bases,
    caught_stealing = p.caught_stealing,
    gdp            = p.gdp,
    sac_flies      = p.sac_flies,
    sac_hits       = p.sac_hits,
    hbp            = p.hbp,
    ibb            = p.ibb
FROM parsed p
WHERE bl.id = p.id
  AND (bl.doubles, bl.triples, bl.home_runs, bl.stolen_bases, bl.caught_stealing,
       bl.gdp, bl.sac_flies, bl.sac_hits, bl.hbp, bl.ibb)
      IS DISTINCT FROM
      (p.doubles, p.triples, p.home_runs, p.stolen_bases, p.caught_stealing,
       p.gdp, p.sac_flies, p.sac_hits, p.hbp, p.ibb);

COMMIT;
