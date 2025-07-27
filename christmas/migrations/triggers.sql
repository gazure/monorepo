-- Function to ensure bidirectional relationship
-- CREATE OR REPLACE FUNCTION ensure_bidirectional_relationship()
-- RETURNS TRIGGER AS $$
-- BEGIN
--     -- Insert the reverse relationship if it doesn't exist
--     INSERT INTO participant_relationship (participant1_id, participant2_id, relationship_type)
--     VALUES (NEW.participant2_id, NEW.participant1_id, NEW.relationship_type)
--     ON CONFLICT (participant1_id, participant2_id, relationship_type) DO NOTHING;

--     RETURN NEW;
-- END;
-- $$ LANGUAGE plpgsql;

-- CREATE TRIGGER trigger_bidirectional_relationship
--     AFTER INSERT ON participant_relationship
--     FOR EACH ROW
--     EXECUTE FUNCTION ensure_bidirectional_relationship();
