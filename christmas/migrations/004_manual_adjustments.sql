-- Hand adjustments to a draw.
--
-- A swap is recorded the same way a re-draw is: a new revision, with the
-- previous one left intact. These two columns are what distinguish the two, so
-- an adjusted year can say so instead of quietly presenting itself as something
-- the solver produced.
ALTER TABLE exchange ADD COLUMN adjusted_from INTEGER REFERENCES exchange(id) ON DELETE SET NULL;
ALTER TABLE exchange ADD COLUMN adjustment_note TEXT;

COMMENT ON COLUMN exchange.adjusted_from IS
    'The revision this one was hand-edited from. NULL for draws the solver produced.';
COMMENT ON COLUMN exchange.adjustment_note IS
    'What the edit was, in words, for the manage page and the audit trail.';
