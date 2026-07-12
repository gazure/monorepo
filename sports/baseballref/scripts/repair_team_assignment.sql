-- Repair batting_lines.team_id / pitching_lines.team_id for games scraped
-- before the table-order fix (parsers assigned teams by matching the team
-- code against table ids like "LosAngelesDodgersbatting", which usually
-- fails and fell back to the home code, or to the away code for Dodgers
-- tables).
--
-- Method: within a game, rows were inserted in parse order (ids ascend), the
-- away table first then the home table, and batting_order/pitch_order restart
-- at 1 per table. Splitting each game's rows at the order-reset yields the two
-- table blocks. Each block's true team comes from a majority vote of its
-- members' play_by_play batting teams (or fielding teams, for pitchers) —
-- which also handles relocated "home" games where the home table renders
-- first. Games whose blocks can't be resolved are listed for re-scraping.
--
-- Run: psql "$SPORTS_DATABASE_URL" -f repair_team_assignment.sql

BEGIN;

-- ---------------------------------------------------------------------------
-- Batting lines: block membership via batting_order resets
-- ---------------------------------------------------------------------------
CREATE TEMP TABLE bat_blocks AS
SELECT id,
       game_id,
       team_id,
       player_id,
       sum(is_reset) OVER (PARTITION BY game_id ORDER BY id) AS block
FROM (
    SELECT bl.id, bl.game_id, bl.team_id, bl.player_id,
           CASE WHEN bl.batting_order <= lag(bl.batting_order) OVER (PARTITION BY bl.game_id ORDER BY bl.id)
                THEN 1 ELSE 0 END AS is_reset
    FROM batting_lines bl
) x;

-- Batter -> team from play-by-play (mode over events, ties broken arbitrarily)
CREATE TEMP TABLE pbp_batter_team AS
SELECT DISTINCT ON (game_id, batter_id) game_id, batter_id, batting_team_id AS team_id
FROM (
    SELECT game_id, batter_id, batting_team_id, count(*) AS n
    FROM play_by_play
    GROUP BY 1, 2, 3
) t
ORDER BY game_id, batter_id, n DESC;

-- Majority vote per block
CREATE TEMP TABLE bat_block_team AS
SELECT game_id, block, team_id AS voted_team_id, votes, total_votes
FROM (
    SELECT b.game_id, b.block, p.team_id,
           count(*) AS votes,
           sum(count(*)) OVER (PARTITION BY b.game_id, b.block) AS total_votes,
           row_number() OVER (PARTITION BY b.game_id, b.block ORDER BY count(*) DESC) AS rk
    FROM bat_blocks b
    JOIN pbp_batter_team p ON p.game_id = b.game_id AND p.batter_id = b.player_id
    GROUP BY 1, 2, 3
) v
WHERE rk = 1;

-- Resolvable games: exactly 2 blocks, each voted unanimously-enough (>= 80%),
-- blocks voted for the two distinct teams of the game
CREATE TEMP TABLE bat_resolved AS
SELECT g.id AS game_id,
       min(bt.voted_team_id) FILTER (WHERE bt.block = 0) AS block0_team,
       min(bt.voted_team_id) FILTER (WHERE bt.block = 1) AS block1_team
FROM games g
JOIN bat_block_team bt ON bt.game_id = g.id
WHERE bt.votes::float8 / bt.total_votes::float8 >= 0.8
GROUP BY g.id
HAVING count(DISTINCT bt.block) = 2
   AND count(*) = 2
   AND min(bt.voted_team_id) FILTER (WHERE bt.block = 0)
       <> min(bt.voted_team_id) FILTER (WHERE bt.block = 1)
   AND min(bt.voted_team_id) FILTER (WHERE bt.block = 0) IN (g.away_team_id, g.home_team_id)
   AND min(bt.voted_team_id) FILTER (WHERE bt.block = 1) IN (g.away_team_id, g.home_team_id);

-- Games with any misassigned row, restricted to resolvable ones
CREATE TEMP TABLE bat_repairs AS
SELECT bb.id, bb.game_id, CASE WHEN bb.block = 0 THEN r.block0_team ELSE r.block1_team END AS new_team_id
FROM bat_blocks bb
JOIN bat_resolved r ON r.game_id = bb.game_id
WHERE bb.team_id <> CASE WHEN bb.block = 0 THEN r.block0_team ELSE r.block1_team END;

SELECT count(*) AS batting_rows_to_fix, count(DISTINCT game_id) AS batting_games_to_fix FROM bat_repairs;

UPDATE batting_lines bl
SET team_id = br.new_team_id
FROM bat_repairs br
WHERE bl.id = br.id;

-- ---------------------------------------------------------------------------
-- Pitching lines: same approach via pitch_order resets; a pitcher's team is
-- the fielding side of their play-by-play events
-- ---------------------------------------------------------------------------
CREATE TEMP TABLE pit_blocks AS
SELECT id,
       game_id,
       team_id,
       player_id,
       sum(is_reset) OVER (PARTITION BY game_id ORDER BY id) AS block
FROM (
    -- Orders strictly increase within a table, so an equal-or-lower order
    -- marks the second table (covers one-pitcher complete games: 1, 1).
    SELECT pl.id, pl.game_id, pl.team_id, pl.player_id,
           CASE WHEN pl.pitch_order <= lag(pl.pitch_order) OVER (PARTITION BY pl.game_id ORDER BY pl.id)
                THEN 1 ELSE 0 END AS is_reset
    FROM pitching_lines pl
) x;

CREATE TEMP TABLE pbp_pitcher_team AS
SELECT DISTINCT ON (p.game_id, p.pitcher_id) p.game_id, p.pitcher_id,
       CASE WHEN p.batting_team_id = g.home_team_id THEN g.away_team_id ELSE g.home_team_id END AS team_id
FROM (
    SELECT game_id, pitcher_id, batting_team_id, count(*) AS n
    FROM play_by_play
    GROUP BY 1, 2, 3
) p
JOIN games g ON g.id = p.game_id
ORDER BY p.game_id, p.pitcher_id, p.n DESC;

CREATE TEMP TABLE pit_block_team AS
SELECT game_id, block, team_id AS voted_team_id, votes, total_votes
FROM (
    SELECT b.game_id, b.block, p.team_id,
           count(*) AS votes,
           sum(count(*)) OVER (PARTITION BY b.game_id, b.block) AS total_votes,
           row_number() OVER (PARTITION BY b.game_id, b.block ORDER BY count(*) DESC) AS rk
    FROM pit_blocks b
    JOIN pbp_pitcher_team p ON p.game_id = b.game_id AND p.pitcher_id = b.player_id
    GROUP BY 1, 2, 3
) v
WHERE rk = 1;

CREATE TEMP TABLE pit_resolved AS
SELECT g.id AS game_id,
       min(bt.voted_team_id) FILTER (WHERE bt.block = 0) AS block0_team,
       min(bt.voted_team_id) FILTER (WHERE bt.block = 1) AS block1_team
FROM games g
JOIN pit_block_team bt ON bt.game_id = g.id
WHERE bt.votes::float8 / bt.total_votes::float8 >= 0.8
GROUP BY g.id
HAVING count(DISTINCT bt.block) = 2
   AND count(*) = 2
   AND min(bt.voted_team_id) FILTER (WHERE bt.block = 0)
       <> min(bt.voted_team_id) FILTER (WHERE bt.block = 1)
   AND min(bt.voted_team_id) FILTER (WHERE bt.block = 0) IN (g.away_team_id, g.home_team_id)
   AND min(bt.voted_team_id) FILTER (WHERE bt.block = 1) IN (g.away_team_id, g.home_team_id);

CREATE TEMP TABLE pit_repairs AS
SELECT pb.id, pb.game_id, CASE WHEN pb.block = 0 THEN r.block0_team ELSE r.block1_team END AS new_team_id
FROM pit_blocks pb
JOIN pit_resolved r ON r.game_id = pb.game_id
WHERE pb.team_id <> CASE WHEN pb.block = 0 THEN r.block0_team ELSE r.block1_team END;

SELECT count(*) AS pitching_rows_to_fix, count(DISTINCT game_id) AS pitching_games_to_fix FROM pit_repairs;

UPDATE pitching_lines pl
SET team_id = pr.new_team_id
FROM pit_repairs pr
WHERE pl.id = pr.id;

-- ---------------------------------------------------------------------------
-- Unresolvable games (need re-scrape): still misassigned after repair
-- ---------------------------------------------------------------------------
CREATE TEMP TABLE still_bad AS
SELECT g.id, g.bbref_game_id
FROM games g
WHERE (SELECT count(*) FILTER (WHERE bl.team_id = g.away_team_id) FROM batting_lines bl WHERE bl.game_id = g.id) < 5
   OR (SELECT count(*) FILTER (WHERE bl.team_id = g.home_team_id) FROM batting_lines bl WHERE bl.game_id = g.id) < 5
   OR (SELECT count(*) FILTER (WHERE pl.team_id = g.away_team_id) FROM pitching_lines pl WHERE pl.game_id = g.id) < 1
   OR (SELECT count(*) FILTER (WHERE pl.team_id = g.home_team_id) FROM pitching_lines pl WHERE pl.game_id = g.id) < 1;

SELECT count(*) AS games_still_bad FROM still_bad;

COMMIT;
