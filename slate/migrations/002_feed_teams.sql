-- A configured feed names a team the way ESPN's schedule endpoint wants it
-- (`usa.seattle` for MLS, `wash` for NCAAF), but games are stored under the
-- abbreviation ESPN prints on each matchup (`SEA`, `WASH`). Sync records the
-- pairing here so a feed URL can use either form.

CREATE TABLE feed_teams (
    league      TEXT NOT NULL,
    team        TEXT NOT NULL,
    abbr        TEXT NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (league, team)
);
