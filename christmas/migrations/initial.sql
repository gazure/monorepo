
-- Participants table - stores basic participant information
CREATE TABLE participant (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Exchange table - stores exchange information
CREATE TABLE exchange (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    year INTEGER NOT NULL,
    status VARCHAR(50) DEFAULT 'planning', -- planning, open, closed, completed
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Junction table for many-to-many relationship between exchanges and participants
CREATE TABLE exchange_participants (
    id SERIAL PRIMARY KEY,
    exchange_id INTEGER NOT NULL REFERENCES exchange(id) ON DELETE CASCADE,
    participant_id INTEGER NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    preferences TEXT, -- gift preferences/wishlist
    address TEXT, -- shipping address for this exchange
    UNIQUE(exchange_id, participant_id)
);

-- Participant relationships - tracks existing ties (spouses, family, etc.)
CREATE TABLE participant_relationships (
    id SERIAL PRIMARY KEY,
    participant1_id INTEGER NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    participant2_id INTEGER NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    relationship_type VARCHAR(50) NOT NULL, -- 'spouse', 'family', 'roommate', etc.
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT no_self_relationship CHECK (participant1_id != participant2_id),
    CONSTRAINT unique_relationship UNIQUE(participant1_id, participant2_id, relationship_type)
);

-- Exchange matches - tracks who gives to whom in each exchange
CREATE TABLE exchange_matches (
    id SERIAL PRIMARY KEY,
    exchange_id INTEGER NOT NULL REFERENCES exchange(id) ON DELETE CASCADE,
    giver_id INTEGER NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    receiver_id INTEGER NOT NULL REFERENCES participants(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    CONSTRAINT no_self_giving CHECK (giver_id != receiver_id),
    CONSTRAINT unique_giver_per_exchange UNIQUE(exchange_id, giver_id),
    CONSTRAINT unique_receiver_per_exchange UNIQUE(exchange_id, receiver_id)
);

-- Indexes for better query performance
CREATE INDEX idx_exchange_participants_exchange_id ON exchange_participants(exchange_id);
CREATE INDEX idx_exchange_participants_participant_id ON exchange_participants(participant_id);
CREATE INDEX idx_participant_relationships_participant1 ON participant_relationships(participant1_id);
CREATE INDEX idx_participant_relationships_participant2 ON participant_relationships(participant2_id);
CREATE INDEX idx_exchange_matches_exchange_id ON exchange_matches(exchange_id);
CREATE INDEX idx_exchange_matches_giver_id ON exchange_matches(giver_id);
CREATE INDEX idx_exchange_matches_receiver_id ON exchange_matches(receiver_id);

-- Function to ensure bidirectional relationships
CREATE OR REPLACE FUNCTION ensure_bidirectional_relationship()
RETURNS TRIGGER AS $$
BEGIN
    -- Insert the reverse relationship if it doesn't exist
    INSERT INTO participant_relationships (participant1_id, participant2_id, relationship_type)
    VALUES (NEW.participant2_id, NEW.participant1_id, NEW.relationship_type)
    ON CONFLICT (participant1_id, participant2_id, relationship_type) DO NOTHING;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_bidirectional_relationship
    AFTER INSERT ON participant_relationships
    FOR EACH ROW
    EXECUTE FUNCTION ensure_bidirectional_relationship();
