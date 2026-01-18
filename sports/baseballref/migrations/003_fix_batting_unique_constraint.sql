-- Fix unique constraint to allow players on both teams (e.g., Danny Jansen traded mid-game)
-- Change from (game_id, player_id) to (game_id, player_id, team_id)

ALTER TABLE batting_lines DROP CONSTRAINT batting_lines_game_id_player_id_key;
ALTER TABLE batting_lines ADD CONSTRAINT batting_lines_game_id_player_id_team_id_key 
    UNIQUE(game_id, player_id, team_id);

ALTER TABLE pitching_lines DROP CONSTRAINT pitching_lines_game_id_player_id_key;
ALTER TABLE pitching_lines ADD CONSTRAINT pitching_lines_game_id_player_id_team_id_key 
    UNIQUE(game_id, player_id, team_id);
