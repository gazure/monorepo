-- Migrate to JSONB snapshots for exchanges
-- This preserves historical exchange data independent of the active participant pool

-- Add JSONB columns to exchange table
ALTER TABLE exchange ADD COLUMN participants JSONB;
ALTER TABLE exchange ADD COLUMN exclusions JSONB;
ALTER TABLE exchange ADD COLUMN pairings JSONB;

-- Migrate existing pairing data to JSONB format
UPDATE exchange e SET 
    participants = COALESCE(
        (SELECT jsonb_agg(DISTINCT p.name ORDER BY p.name)
         FROM pairing pa
         JOIN participant p ON p.id IN (pa.giver_id, pa.receiver_id)
         WHERE pa.exchange_id = e.id),
        '[]'::jsonb
    ),
    exclusions = '[]'::jsonb,
    pairings = COALESCE(
        (SELECT jsonb_agg(jsonb_build_object('giver', pg.name, 'receiver', pr.name))
         FROM pairing pa
         JOIN participant pg ON pa.giver_id = pg.id
         JOIN participant pr ON pa.receiver_id = pr.id
         WHERE pa.exchange_id = e.id),
        '[]'::jsonb
    );

-- Set NOT NULL constraints after migration
ALTER TABLE exchange ALTER COLUMN participants SET NOT NULL;
ALTER TABLE exchange ALTER COLUMN participants SET DEFAULT '[]'::jsonb;
ALTER TABLE exchange ALTER COLUMN exclusions SET NOT NULL;
ALTER TABLE exchange ALTER COLUMN exclusions SET DEFAULT '[]'::jsonb;
ALTER TABLE exchange ALTER COLUMN pairings SET NOT NULL;
ALTER TABLE exchange ALTER COLUMN pairings SET DEFAULT '[]'::jsonb;

-- Drop the pairing table (no longer needed)
DROP TABLE pairing;
