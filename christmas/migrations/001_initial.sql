-- Participants in the exchange
CREATE TABLE participant (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Exclusions: pairs of participants who should not be matched
CREATE TABLE exclusion (
    id SERIAL PRIMARY KEY,
    participant_a_id INTEGER NOT NULL REFERENCES participant(id) ON DELETE CASCADE,
    participant_b_id INTEGER NOT NULL REFERENCES participant(id) ON DELETE CASCADE,
    reason VARCHAR(255),
    UNIQUE(participant_a_id, participant_b_id),
    CHECK(participant_a_id < participant_b_id)
);

-- Letters that are excluded from selection
CREATE TABLE excluded_letter (
    letter CHAR(1) PRIMARY KEY CHECK(letter ~ '^[A-Z]$')
);

-- Exchange results (historical record)
CREATE TABLE exchange (
    id SERIAL PRIMARY KEY,
    year INTEGER NOT NULL UNIQUE,
    letter CHAR(1),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Pairings for each exchange
CREATE TABLE pairing (
    id SERIAL PRIMARY KEY,
    exchange_id INTEGER NOT NULL REFERENCES exchange(id) ON DELETE CASCADE,
    giver_id INTEGER NOT NULL REFERENCES participant(id),
    receiver_id INTEGER NOT NULL REFERENCES participant(id),
    UNIQUE(exchange_id, giver_id),
    UNIQUE(exchange_id, receiver_id),
    CHECK(giver_id != receiver_id)
);

-- Index for faster exclusion lookups
CREATE INDEX idx_exclusion_participants ON exclusion(participant_a_id, participant_b_id);
