-- Baseball Reference Scraper Schema
-- Initial migration for box score data

-- Teams (reference data)
CREATE TABLE teams (
    id SERIAL PRIMARY KEY,
    code VARCHAR(3) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Players (reference data, upserted as discovered)
CREATE TABLE players (
    id SERIAL PRIMARY KEY,
    bbref_id VARCHAR(20) UNIQUE NOT NULL,
    name VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Games
CREATE TABLE games (
    id SERIAL PRIMARY KEY,
    bbref_game_id VARCHAR(20) UNIQUE NOT NULL,
    game_date DATE NOT NULL,
    start_time VARCHAR(20),
    venue VARCHAR(100),
    attendance INTEGER,
    duration_minutes INTEGER,
    weather VARCHAR(200),
    is_night_game BOOLEAN,
    is_artificial_turf BOOLEAN,
    home_team_id INTEGER NOT NULL REFERENCES teams(id),
    away_team_id INTEGER NOT NULL REFERENCES teams(id),
    home_score INTEGER,
    away_score INTEGER,
    winning_pitcher_id INTEGER REFERENCES players(id),
    losing_pitcher_id INTEGER REFERENCES players(id),
    save_pitcher_id INTEGER REFERENCES players(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Umpires (many-to-many with games)
CREATE TABLE game_umpires (
    id SERIAL PRIMARY KEY,
    game_id INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    position VARCHAR(10) NOT NULL,
    name VARCHAR(100) NOT NULL
);

-- Line score (runs per inning)
CREATE TABLE game_line_scores (
    id SERIAL PRIMARY KEY,
    game_id INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    team_id INTEGER NOT NULL REFERENCES teams(id),
    is_home BOOLEAN NOT NULL,
    inning INTEGER NOT NULL,
    runs INTEGER NOT NULL,
    UNIQUE(game_id, team_id, inning)
);

-- Batting lines
CREATE TABLE batting_lines (
    id SERIAL PRIMARY KEY,
    game_id INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    player_id INTEGER NOT NULL REFERENCES players(id),
    team_id INTEGER NOT NULL REFERENCES teams(id),
    batting_order INTEGER,
    position VARCHAR(10),
    ab INTEGER,
    r INTEGER,
    h INTEGER,
    rbi INTEGER,
    bb INTEGER,
    so INTEGER,
    pa INTEGER,
    batting_avg DECIMAL(4,3),
    obp DECIMAL(4,3),
    slg DECIMAL(4,3),
    ops DECIMAL(4,3),
    pitches_seen INTEGER,
    strikes_seen INTEGER,
    wpa DECIMAL(6,3),
    ali DECIMAL(5,2),
    wpa_pos DECIMAL(6,3),
    wpa_neg DECIMAL(6,3),
    cwpa DECIMAL(6,3),
    acli DECIMAL(5,2),
    re24 DECIMAL(5,1),
    po INTEGER,
    a INTEGER,
    details VARCHAR(200),
    UNIQUE(game_id, player_id)
);

-- Pitching lines
CREATE TABLE pitching_lines (
    id SERIAL PRIMARY KEY,
    game_id INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    player_id INTEGER NOT NULL REFERENCES players(id),
    team_id INTEGER NOT NULL REFERENCES teams(id),
    pitch_order INTEGER,
    decision VARCHAR(10),
    ip DECIMAL(4,1),
    h INTEGER,
    r INTEGER,
    er INTEGER,
    bb INTEGER,
    so INTEGER,
    hr INTEGER,
    era DECIMAL(5,2),
    batters_faced INTEGER,
    pitches INTEGER,
    strikes INTEGER,
    strikes_contact INTEGER,
    strikes_swinging INTEGER,
    strikes_looking INTEGER,
    ground_balls INTEGER,
    fly_balls INTEGER,
    line_drives INTEGER,
    game_score INTEGER,
    inherited_runners INTEGER,
    inherited_scored INTEGER,
    wpa DECIMAL(6,3),
    ali DECIMAL(5,2),
    cwpa DECIMAL(6,3),
    acli DECIMAL(5,2),
    re24 DECIMAL(5,1),
    UNIQUE(game_id, player_id)
);

-- Play by play (one row per plate appearance)
CREATE TABLE play_by_play (
    id SERIAL PRIMARY KEY,
    game_id INTEGER NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    event_num INTEGER NOT NULL,
    inning INTEGER NOT NULL,
    is_bottom BOOLEAN NOT NULL,
    batting_team_id INTEGER NOT NULL REFERENCES teams(id),
    batter_id INTEGER NOT NULL REFERENCES players(id),
    pitcher_id INTEGER NOT NULL REFERENCES players(id),
    outs_before INTEGER,
    runners_before VARCHAR(10),
    score_batting_team INTEGER,
    score_fielding_team INTEGER,
    pitch_sequence VARCHAR(50),
    pitch_count INTEGER,
    runs_on_play INTEGER,
    outs_on_play INTEGER,
    wpa DECIMAL(6,3),
    win_expectancy_after DECIMAL(5,3),
    play_description TEXT,
    UNIQUE(game_id, event_num)
);

-- Indexes for common queries
CREATE INDEX idx_games_date ON games(game_date);
CREATE INDEX idx_games_home_team ON games(home_team_id);
CREATE INDEX idx_games_away_team ON games(away_team_id);
CREATE INDEX idx_batting_lines_player ON batting_lines(player_id);
CREATE INDEX idx_batting_lines_game ON batting_lines(game_id);
CREATE INDEX idx_pitching_lines_player ON pitching_lines(player_id);
CREATE INDEX idx_pitching_lines_game ON pitching_lines(game_id);
CREATE INDEX idx_play_by_play_game ON play_by_play(game_id);
CREATE INDEX idx_play_by_play_batter ON play_by_play(batter_id);
CREATE INDEX idx_play_by_play_pitcher ON play_by_play(pitcher_id);
CREATE INDEX idx_game_umpires_game ON game_umpires(game_id);
CREATE INDEX idx_game_line_scores_game ON game_line_scores(game_id);
