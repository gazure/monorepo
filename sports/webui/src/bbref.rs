//! Outbound baseball-reference.com links

pub fn player_url(bbref_id: &str) -> String {
    let first = bbref_id.chars().next().unwrap_or('x');
    format!("https://www.baseball-reference.com/players/{first}/{bbref_id}.shtml")
}

pub fn box_url(bbref_game_id: &str) -> String {
    // First three characters of a bbref game id are the home team code
    let home = bbref_game_id.get(..3).unwrap_or("xxx");
    format!("https://www.baseball-reference.com/boxes/{home}/{bbref_game_id}.shtml")
}
