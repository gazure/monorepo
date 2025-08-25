
-- participant table - stores basic participant information
CREATE TABLE IF NOT EXISTS participant (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Exchange table - stores exchange information
CREATE TABLE IF NOT EXISTS exchange (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    year INTEGER NOT NULL,
    status VARCHAR(50) DEFAULT 'planning', -- planning, open, closed, completed
    letters VARCHAR(26) DEFAULT 'ABCDEFGHIJKLMNOPQRSTUVWXYZ',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Junction table for many-to-many relationship between exchange and participant
CREATE TABLE IF NOT EXISTS exchange_participant (
    id SERIAL PRIMARY KEY,
    exchange_id INTEGER NOT NULL REFERENCES exchange(id) ON DELETE CASCADE,
    participant_id INTEGER NOT NULL REFERENCES participant(id) ON DELETE CASCADE,
    preferences TEXT, -- gift preferences/wishlist
    address TEXT, -- shipping address for this exchange
    UNIQUE(exchange_id, participant_id)
);

-- Participant relationship - tracks existing ties (spouses, family, etc.)
CREATE TABLE IF NOT EXISTS participant_relationship (
    id SERIAL PRIMARY KEY,
    participant1_id INTEGER NOT NULL REFERENCES participant(id) ON DELETE CASCADE,
    participant2_id INTEGER NOT NULL REFERENCES participant(id) ON DELETE CASCADE,
    relationship_type VARCHAR(50) NOT NULL, -- 'spouse', 'family', 'roommate', etc.
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT no_self_relationship CHECK (participant1_id != participant2_id),
    CONSTRAINT unique_relationship UNIQUE(participant1_id, participant2_id, relationship_type)
);

-- Exchange matches - tracks who gives to whom in each exchange
CREATE TABLE IF NOT EXISTS exchange_match (
    id SERIAL PRIMARY KEY,
    exchange_id INTEGER NOT NULL REFERENCES exchange(id) ON DELETE CASCADE,
    giver_id INTEGER NOT NULL REFERENCES participant(id) ON DELETE CASCADE,
    receiver_id INTEGER NOT NULL REFERENCES participant(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT no_self_giving CHECK (giver_id != receiver_id),
    CONSTRAINT unique_giver_per_exchange UNIQUE(exchange_id, giver_id),
    CONSTRAINT unique_receiver_per_exchange UNIQUE(exchange_id, receiver_id)
);

-- Indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_exchange_participant_exchange_id ON exchange_participant(exchange_id);
CREATE INDEX IF NOT EXISTS idx_exchange_participant_participant_id ON exchange_participant(participant_id);
CREATE INDEX IF NOT EXISTS idx_participant_relationship_participant1 ON participant_relationship(participant1_id);
CREATE INDEX IF NOT EXISTS idx_participant_relationship_participant2 ON participant_relationship(participant2_id);
CREATE INDEX IF NOT EXISTS idx_exchange_match_exchange_id ON exchange_match(exchange_id);
CREATE INDEX IF NOT EXISTS idx_exchange_match_giver_id ON exchange_match(giver_id);
CREATE INDEX IF NOT EXISTS idx_exchange_match_receiver_id ON exchange_match(receiver_id);
