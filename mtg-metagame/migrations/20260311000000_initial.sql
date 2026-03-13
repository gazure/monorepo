CREATE TABLE IF NOT EXISTS tournament (
    id SERIAL PRIMARY KEY,
    goldfish_id INTEGER NOT NULL UNIQUE,
    name TEXT NOT NULL,
    format TEXT NOT NULL,
    date DATE NOT NULL,
    url TEXT NOT NULL,
    scraped_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS archetype (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    format TEXT NOT NULL,
    url TEXT,
    UNIQUE (name, format)
);

CREATE TABLE IF NOT EXISTS deck (
    id SERIAL PRIMARY KEY,
    goldfish_id INTEGER NOT NULL UNIQUE,
    tournament_id INTEGER REFERENCES tournament(id),
    archetype_id INTEGER REFERENCES archetype(id),
    player_name TEXT,
    placement TEXT,
    format TEXT NOT NULL,
    date DATE,
    url TEXT NOT NULL,
    scraped_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS deck_card (
    id SERIAL PRIMARY KEY,
    deck_id INTEGER NOT NULL REFERENCES deck(id) ON DELETE CASCADE,
    card_name TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    is_sideboard BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_deck_format ON deck (format);
CREATE INDEX IF NOT EXISTS idx_deck_archetype ON deck (archetype_id);
CREATE INDEX IF NOT EXISTS idx_deck_card_name ON deck_card (card_name);
CREATE INDEX IF NOT EXISTS idx_deck_card_deck_id ON deck_card (deck_id);
CREATE INDEX IF NOT EXISTS idx_archetype_format ON archetype (format);
CREATE INDEX IF NOT EXISTS idx_tournament_format_date ON tournament (format, date);
