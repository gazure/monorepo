-- Increase position column to accommodate compound positions (e.g., "PH-DH-SS-2B")
-- Original limit of VARCHAR(10) was too small for 4+ position changes

ALTER TABLE batting_lines ALTER COLUMN position TYPE VARCHAR(30);
