-- slate: sports schedule calendar feeds
--
-- `games` holds the current known state of every game we have ever observed.
-- `game_revisions` is an append-only log of every field change we have seen,
-- which is what lets us emit a correct iCalendar SEQUENCE and answer
-- "when did this game move, and what did it move from?"

CREATE TABLE games (
    id          BIGSERIAL PRIMARY KEY,
    league      TEXT NOT NULL,
    espn_id     TEXT NOT NULL,
    season      INTEGER NOT NULL,
    week        INTEGER,
    name        TEXT NOT NULL,
    short_name  TEXT NOT NULL,
    home_abbr   TEXT NOT NULL,
    home_name   TEXT NOT NULL,
    away_abbr   TEXT NOT NULL,
    away_name   TEXT NOT NULL,

    -- Calendar-significant fields. A change to any of these bumps `sequence`.
    kickoff     TIMESTAMPTZ NOT NULL,
    time_tbd    BOOLEAN NOT NULL DEFAULT FALSE,
    venue       TEXT,
    city        TEXT,
    status      TEXT NOT NULL,

    -- Informational: tracked and logged, but does not bump `sequence`.
    broadcast   TEXT,

    sequence    INTEGER NOT NULL DEFAULT 0,
    first_seen  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen   TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (league, espn_id)
);

CREATE INDEX games_league_kickoff_idx ON games (league, kickoff);
CREATE INDEX games_team_idx ON games (league, home_abbr, away_abbr);

CREATE TABLE game_revisions (
    id           BIGSERIAL PRIMARY KEY,
    game_id      BIGINT NOT NULL REFERENCES games (id) ON DELETE CASCADE,
    observed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    field        TEXT NOT NULL,
    old_value    TEXT,
    new_value    TEXT,
    significant  BOOLEAN NOT NULL
);

CREATE INDEX game_revisions_game_idx ON game_revisions (game_id, observed_at DESC);
CREATE INDEX game_revisions_recent_idx ON game_revisions (observed_at DESC);
