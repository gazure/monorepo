-- Add pitch_order to unique constraint to handle cases where a pitcher
-- pitches multiple separate times in the same game (e.g., pitcher moves to 1B then back to pitcher)
-- Example: https://www.baseball-reference.com/boxes/TBA/TBA201907240.shtml

ALTER TABLE pitching_lines DROP CONSTRAINT pitching_lines_game_id_player_id_team_id_key;
ALTER TABLE pitching_lines ADD CONSTRAINT pitching_lines_game_id_player_id_team_id_pitch_order_key
    UNIQUE(game_id, player_id, team_id, pitch_order);
