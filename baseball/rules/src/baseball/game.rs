use std::fmt::Display;

use super::{
    inning::{HalfInning, HalfInningResult, InningHalf},
    lineup::BattingPosition,
    plate_appearance::PitchOutcome,
    runs::Runs,
};

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum InningNumber {
    #[default]
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    Sixth,
    Seventh,
    Eighth,
    Ninth,
    Extra(u8), // For extra innings (10th, 11th, etc.)
}

impl InningNumber {
    pub fn next(self) -> InningNumber {
        use InningNumber::*;
        match self {
            First => Second,
            Second => Third,
            Third => Fourth,
            Fourth => Fifth,
            Fifth => Sixth,
            Sixth => Seventh,
            Seventh => Eighth,
            Eighth => Ninth,
            Ninth => Extra(10),
            Extra(n) => Extra(n + 1),
        }
    }

    pub fn as_number(self) -> u8 {
        use InningNumber::*;
        match self {
            First => 1,
            Second => 2,
            Third => 3,
            Fourth => 4,
            Fifth => 5,
            Sixth => 6,
            Seventh => 7,
            Eighth => 8,
            Ninth => 9,
            Extra(n) => n,
        }
    }

    pub fn is_extra(&self) -> bool {
        matches!(self, InningNumber::Extra(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameScore {
    away: Runs,
    home: Runs,
}

impl Display for GameScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Away: {} - Home: {}", self.away, self.home)
    }
}

impl GameScore {
    pub fn new() -> Self {
        GameScore { away: 0, home: 0 }
    }

    pub fn away(&self) -> Runs {
        self.away
    }

    pub fn home(&self) -> Runs {
        self.home
    }

    pub fn add_away_runs(mut self, runs: Runs) -> Self {
        self.away += runs;
        self
    }

    pub fn add_home_runs(mut self, runs: Runs) -> Self {
        self.home += runs;
        self
    }

    pub fn winner(&self) -> Option<GameWinner> {
        match self.away.cmp(&self.home) {
            std::cmp::Ordering::Greater => Some(GameWinner::Away),
            std::cmp::Ordering::Less => Some(GameWinner::Home),
            std::cmp::Ordering::Equal => None, // Tie
        }
    }
}

impl Default for GameScore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameWinner {
    Away,
    Home,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameSummary {
    final_score: GameScore,
    innings_played: InningNumber,
    winner: GameWinner,
    line_score: LineScore,
}

impl Display for GameSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Final Score: Away: {} - Home: {}",
            self.final_score.away, self.final_score.home
        )
    }
}

impl GameSummary {
    pub fn new(
        final_score: GameScore,
        innings_played: InningNumber,
        winner: GameWinner,
        line_score: LineScore,
    ) -> Self {
        GameSummary {
            final_score,
            innings_played,
            winner,
            line_score,
        }
    }

    pub fn line_score(&self) -> &LineScore {
        &self.line_score
    }

    pub fn final_score(&self) -> GameScore {
        self.final_score
    }

    pub fn innings_played(&self) -> InningNumber {
        self.innings_played
    }

    pub fn winner(&self) -> GameWinner {
        self.winner
    }
}

/// Runs per half inning plus hit totals — everything a box score line needs.
///
/// The two run vectors can legitimately differ in length: a game that ends after
/// the top of the ninth never gives the home team a ninth at-bat, which is the
/// `X` you see on a real scoreboard.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineScore {
    away_innings: Vec<Runs>,
    home_innings: Vec<Runs>,
    away_hits: u16,
    home_hits: u16,
}

impl LineScore {
    pub fn away_innings(&self) -> &[Runs] {
        &self.away_innings
    }

    pub fn home_innings(&self) -> &[Runs] {
        &self.home_innings
    }

    pub fn away_hits(&self) -> u16 {
        self.away_hits
    }

    pub fn home_hits(&self) -> u16 {
        self.home_hits
    }

    /// Number of columns the scoreboard needs, i.e. the longer of the two halves.
    pub fn columns(&self) -> usize {
        self.away_innings.len().max(self.home_innings.len())
    }

    fn record_half(&mut self, half: InningHalf, runs: Runs) {
        match half {
            InningHalf::Top => self.away_innings.push(runs),
            InningHalf::Bottom => self.home_innings.push(runs),
        }
    }

    fn record_hit(&mut self, half: InningHalf) {
        match half {
            InningHalf::Top => self.away_hits += 1,
            InningHalf::Bottom => self.home_hits += 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameStatus {
    Inning(InningHalf),
    InningEnd(InningHalf),
    Complete,
}

impl GameStatus {
    pub fn is_bottom(&self) -> bool {
        matches!(self, GameStatus::Inning(InningHalf::Bottom))
    }

    pub fn is_top(&self) -> bool {
        matches!(self, GameStatus::Inning(InningHalf::Top))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    current_inning: InningNumber,
    state: GameStatus,
    score: GameScore,
    current_half_inning: HalfInning,
    away_batting_order: BattingPosition,
    home_batting_order: BattingPosition,
    line_score: LineScore,
}

impl Display for Game {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} Score: {}", self.inning_description(), self.score())
    }
}

impl Game {
    pub fn new() -> Self {
        Game {
            current_inning: InningNumber::First,
            state: GameStatus::Inning(InningHalf::Top),
            score: GameScore::new(),
            current_half_inning: HalfInning::new(InningHalf::Top, BattingPosition::First),
            away_batting_order: BattingPosition::First,
            home_batting_order: BattingPosition::First,
            line_score: LineScore::default(),
        }
    }

    pub fn with_batting_orders(away_order: BattingPosition, home_order: BattingPosition) -> Self {
        let first_half = HalfInning::new(InningHalf::Top, away_order);

        Game {
            current_inning: InningNumber::First,
            state: GameStatus::Inning(InningHalf::Top),
            score: GameScore::new(),
            current_half_inning: first_half,
            away_batting_order: away_order,
            home_batting_order: home_order,
            line_score: LineScore::default(),
        }
    }

    pub fn current_inning(&self) -> InningNumber {
        self.current_inning
    }

    pub fn state(&self) -> GameStatus {
        self.state
    }

    pub fn score(&self) -> GameScore {
        self.score
    }

    pub fn current_half_inning(&self) -> &HalfInning {
        &self.current_half_inning
    }

    pub fn line_score(&self) -> &LineScore {
        &self.line_score
    }

    pub fn advance(mut self, outcome: PitchOutcome) -> GameOutcome {
        // Tallied before the half inning advances, while `state` still names the
        // team that is batting.
        if let Some(half) = self.batting_half() {
            let is_hit = match outcome {
                PitchOutcome::HomeRun => true,
                PitchOutcome::InPlay(play) => play.is_hit(),
                _ => false,
            };
            if is_hit {
                self.line_score.record_hit(half);
            }
        }

        match self.current_half_inning.advance(outcome) {
            HalfInningResult::InProgress(half_inning) => {
                self.current_half_inning = half_inning;
                let pending_runs = self.current_half_inning.runs_scored();
                // A walk-off ends the game the instant the run scores, without
                // waiting for the third out. This applies in extra innings too,
                // not just the ninth.
                if self.is_walkoff_window() && self.should_end_game(pending_runs) {
                    self.complete_half_inning(pending_runs);
                    return self.finish();
                }

                GameOutcome::InProgress(self)
            }

            HalfInningResult::Complete(summary) => {
                // Half inning completed, update score and advance
                self.complete_half_inning(summary.runs_scored());

                // Check if game should end
                if self.should_end_game(0) {
                    return self.finish();
                }

                // Start next half inning
                self = self.start_next_half();
                GameOutcome::InProgress(self)
            }
        }
    }

    /// Which team is at bat right now, or `None` between halves and after the
    /// final out.
    fn batting_half(&self) -> Option<InningHalf> {
        match self.state {
            GameStatus::Inning(half) => Some(half),
            GameStatus::InningEnd(_) | GameStatus::Complete => None,
        }
    }

    fn complete_half_inning(&mut self, pending_runs: Runs) {
        match self.state {
            GameStatus::Inning(InningHalf::Top) => {
                self.score = self.score.add_away_runs(pending_runs);
                self.line_score.record_half(InningHalf::Top, pending_runs);
                self.state = GameStatus::InningEnd(InningHalf::Top);
            }
            GameStatus::Inning(InningHalf::Bottom) => {
                self.score = self.score.add_home_runs(pending_runs);
                self.line_score.record_half(InningHalf::Bottom, pending_runs);
                self.state = GameStatus::InningEnd(InningHalf::Bottom);
            }
            GameStatus::InningEnd(_) | GameStatus::Complete => {
                // Should not happen
            }
        }
    }

    fn should_end_game(&self, pending_runs: Runs) -> bool {
        // The state represents the NEXT half inning to be played after completing a half
        match self.current_inning {
            // Regular innings 1-8: never end
            InningNumber::First
            | InningNumber::Second
            | InningNumber::Third
            | InningNumber::Fourth
            | InningNumber::Fifth
            | InningNumber::Sixth
            | InningNumber::Seventh
            | InningNumber::Eighth => false,

            // The ninth and every extra inning share one set of ending rules;
            // they used to be spelled out separately, and the extra-innings copy
            // was missing the mid-inning walk-off case.
            InningNumber::Ninth | InningNumber::Extra(_) => match self.state {
                // Visitors have finished batting. If the home team is ahead there
                // is no reason to play the home half.
                GameStatus::InningEnd(InningHalf::Top) => self.score.home() > self.score.away(),
                // Home team has been retired. Anyone ahead has won.
                GameStatus::InningEnd(InningHalf::Bottom) => self.score.home() != self.score.away(),
                // Mid-inning with the home team batting: a run that takes the
                // lead ends it on the spot. This is the walk-off.
                GameStatus::Inning(InningHalf::Bottom) => self.score.home() + pending_runs > self.score.away(),
                GameStatus::Inning(InningHalf::Top) => false,
                GameStatus::Complete => true,
            },
        }
    }

    fn start_next_half(mut self) -> Self {
        let (half, batting_order) = match self.state {
            GameStatus::InningEnd(InningHalf::Top) => (InningHalf::Bottom, self.home_batting_order),
            GameStatus::InningEnd(InningHalf::Bottom) => (InningHalf::Top, self.away_batting_order),
            GameStatus::Inning(_) | GameStatus::Complete => {
                // Should not happen
                return self;
            }
        };

        if let InningHalf::Top = half {
            self.current_inning = self.current_inning.next();
        }

        self.current_half_inning = HalfInning::new(half, batting_order);
        self.state = GameStatus::Inning(half);
        self
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.state, GameStatus::Complete)
    }

    pub fn inning_description(&self) -> String {
        let inning_text = if self.current_inning.is_extra() {
            format!("{}th", self.current_inning.as_number())
        } else {
            match self.current_inning {
                InningNumber::First => "1st".to_string(),
                InningNumber::Second => "2nd".to_string(),
                InningNumber::Third => "3rd".to_string(),
                _ => format!("{}th", self.current_inning.as_number()),
            }
        };

        let half_text = match self.state {
            GameStatus::Inning(InningHalf::Top) => "Top",
            GameStatus::Inning(InningHalf::Bottom) => "Bottom",
            GameStatus::InningEnd(InningHalf::Top) => "Mid",
            GameStatus::InningEnd(InningHalf::Bottom) => "End",
            GameStatus::Complete => return "Game Complete".to_string(),
        };

        format!("{half_text} of the {inning_text}")
    }

    /// The home team is batting in an inning that could end the game, so a run
    /// scoring right now is a walk-off. True for the ninth and every extra
    /// inning; before the ninth there is always more baseball to play.
    fn is_walkoff_window(&self) -> bool {
        let late_enough = self.current_inning == InningNumber::Ninth || self.current_inning.is_extra();
        late_enough && self.state.is_bottom()
    }

    /// Seals the game: marks the state complete and hands back the summary.
    /// `state` is set here so [`Game::is_complete`] agrees with the outcome.
    fn finish(mut self) -> GameOutcome {
        let winner = self.score.winner().expect("a completed game must have a winner");
        self.state = GameStatus::Complete;
        GameOutcome::Complete(GameSummary::new(
            self.score,
            self.current_inning,
            winner,
            self.line_score,
        ))
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameOutcome {
    InProgress(Game),
    Complete(GameSummary),
}

impl Display for GameOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameOutcome::InProgress(game) => write!(f, "{game}"),
            GameOutcome::Complete(summary) => write!(f, "{summary}"),
        }
    }
}

impl GameOutcome {
    pub fn advance(self, outcome: PitchOutcome) -> GameOutcome {
        match self {
            GameOutcome::InProgress(game) => game.advance(outcome),
            GameOutcome::Complete(_) => self, // Already complete, ignore the pitch
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, GameOutcome::Complete(_))
    }

    pub fn game(self) -> Option<Game> {
        match self {
            GameOutcome::InProgress(game) => Some(game),
            GameOutcome::Complete(_) => None,
        }
    }

    pub fn game_ref(&self) -> Option<&Game> {
        match self {
            GameOutcome::InProgress(game) => Some(game),
            GameOutcome::Complete(_) => None,
        }
    }

    pub fn summary(self) -> Option<GameSummary> {
        match self {
            GameOutcome::InProgress(_) => None,
            GameOutcome::Complete(summary) => Some(summary),
        }
    }

    pub fn summary_ref(&self) -> Option<&GameSummary> {
        match self {
            GameOutcome::InProgress(_) => None,
            GameOutcome::Complete(summary) => Some(summary),
        }
    }
}

#[cfg(test)]
mod tests {
    use tracingx::info;

    use super::{
        super::{plate_appearance::PitchOutcome, play::PlayResult},
        *,
    };

    #[test]
    fn test_inning_number_progression() {
        let first = InningNumber::First;
        assert_eq!(first.as_number(), 1);

        let ninth = InningNumber::Ninth;
        let tenth = ninth.next();
        assert_eq!(tenth.as_number(), 10);
        assert!(tenth.is_extra());
    }

    #[test]
    fn test_game_creation() {
        let game = Game::new();
        assert_eq!(game.current_inning(), InningNumber::First);
        assert_eq!(game.state(), GameStatus::Inning(InningHalf::Top));
        assert_eq!(game.score().away(), 0);
        assert_eq!(game.score().home(), 0);
    }

    #[test]
    fn test_game_score_tracking() {
        let mut score = GameScore::new();
        score = score.add_away_runs(3);
        score = score.add_home_runs(2);

        assert_eq!(score.away(), 3);
        assert_eq!(score.home(), 2);
        assert_eq!(score.winner(), Some(GameWinner::Away));
    }

    #[test]
    fn test_simple_half_inning_completion() {
        let game = Game::new();

        // Three quick outs to complete top 1st
        let game = game
            .advance(PitchOutcome::InPlay(PlayResult::Groundout))
            .game()
            .expect("Game should continue")
            .advance(PitchOutcome::InPlay(PlayResult::Groundout))
            .game()
            .expect("Game should continue")
            .advance(PitchOutcome::InPlay(PlayResult::Groundout))
            .game()
            .expect("Game should continue");

        // Should now be bottom of 1st
        assert_eq!(game.state(), GameStatus::Inning(InningHalf::Bottom));
        assert_eq!(game.current_inning(), InningNumber::First);
    }

    #[test]
    fn test_home_run_scoring() {
        let game = Game::new();

        // Home run in top 1st
        let game = game
            .advance(PitchOutcome::HomeRun)
            .game()
            .expect("Game should continue");

        // Complete top 1st with two outs
        let game = game
            .advance(PitchOutcome::InPlay(PlayResult::Groundout))
            .advance(PitchOutcome::InPlay(PlayResult::Groundout))
            .advance(PitchOutcome::InPlay(PlayResult::Groundout))
            .game()
            .expect("Game should continue");

        // Should be bottom 1st with 1-0 score
        assert_eq!(game.state(), GameStatus::Inning(InningHalf::Bottom));
        assert_eq!(game.score().away(), 1);
        assert_eq!(game.score().home(), 0);
    }

    #[test]
    fn test_inning_description() {
        let game = Game::new();
        assert_eq!(game.inning_description(), "Top of the 1st");

        let game = Game::with_batting_orders(BattingPosition::First, BattingPosition::First);
        // Simulate completing top half
        let mut game_state = game;
        game_state.state = GameStatus::Inning(InningHalf::Bottom);
        assert_eq!(game_state.inning_description(), "Bottom of the 1st");

        // Test extra innings
        game_state.current_inning = InningNumber::Extra(12);
        game_state.state = GameStatus::Inning(InningHalf::Top);
        assert_eq!(game_state.inning_description(), "Top of the 12th");
    }

    #[test]
    fn test_game_ending_conditions() {
        let mut game = Game::new();

        // Simulate game state at end of 9th inning
        game.current_inning = InningNumber::Ninth;
        game.state = GameStatus::Inning(InningHalf::Top); // Bottom 9th just completed
        game.score = GameScore::new().add_home_runs(5).add_away_runs(3);

        // Home team ahead, game should end
        game.complete_half_inning(0);
        assert!(game.should_end_game(0));

        // Test tie game - should not end
        game.score = GameScore::new().add_home_runs(3).add_away_runs(3);
        assert!(!game.should_end_game(0));

        // Test home team walk-off run
        game.state = GameStatus::Inning(InningHalf::Bottom);
        game.score = GameScore::new().add_home_runs(3).add_away_runs(3);
        assert!(game.should_end_game(1));

        game.current_inning = InningNumber::Eighth;
        assert!(!game.should_end_game(0));
    }

    // A narrated walk through a whole game. Long on purpose: it doubles as
    // readable documentation of how the pieces compose.
    #[allow(clippy::too_many_lines)]
    #[test]
    fn demo_baseball_game() {
        info!("Starting a new baseball game...");
        let game = Game::new();

        info!("Initial state: {}", game.inning_description());
        info!("Score: Away {} - Home {}", game.score().away(), game.score().home());

        // Simulate first inning
        info!("⚾ Simulating game action...");

        // Start with the advance wrapper
        let mut advance = GameOutcome::InProgress(game);

        // Top 1st: Quick three outs
        info!("🔝 Top 1st Inning:");
        for batter in 1..=3 {
            advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));
            if let Some(_game) = advance.game_ref() {
                info!("  Batter #{}: Out", batter);
            } else {
                info!("  Game ended unexpectedly!");
                return;
            }
        }

        if let Some(game) = advance.game_ref() {
            info!("  Half inning complete!");
            info!("  Current state: {}", game.inning_description());
        }

        // Bottom 1st: Home team scores
        info!("🔽 Bottom 1st Inning:");

        // First batter: Home run
        advance = advance.advance(PitchOutcome::HomeRun);
        if advance.game_ref().is_some() {
            info!("  Batter #1: HOME RUN! 🎉");
            // Score will be updated when half inning completes
        } else {
            info!("  Game ended unexpectedly!");
            return;
        }

        // Next two batters: Outs
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        if advance.game_ref().is_some() {
            info!("  Batter #2: Out");
        } else {
            return;
        }

        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        if advance.game_ref().is_some() {
            info!("  Batter #3: Out");
        } else {
            return;
        }

        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        if advance.game_ref().is_some() {
            info!("  Batter #4: Out");
            info!("  Half inning complete!");
        } else {
            return;
        }

        if let Some(game) = advance.game_ref() {
            info!("📊 After 1 inning:");
            info!("  {}", game.inning_description());
            info!("  Score: Away {} - Home {}", game.score().away(), game.score().home());
        }

        // Fast forward through several innings
        info!("⏭️  Fast forwarding through innings 2-8...");

        while let Some(game) = advance.game_ref() {
            if game.current_inning().as_number() >= 9 {
                break;
            }

            info!(
                "  Starting inning {}: {}",
                game.current_inning().as_number(),
                game.inning_description()
            );

            // Simulate quick half innings (3 outs each)
            for out_num in 1..=6 {
                // 3 outs per half inning, 2 half innings
                advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));
                if let Some(game) = advance.game_ref() {
                    if out_num % 3 == 0 {
                        info!("    Half inning complete: {}", game.inning_description());
                        info!("    Score: Away {} - Home {}", game.score().away(), game.score().home());
                    }
                } else if let Some(summary) = advance.summary_ref() {
                    info!("Game completed early!");
                    info!("Game ended after {} outs in fast forward", out_num);
                    info!(
                        "Final Score: Away {} - Home {}",
                        summary.final_score().away(),
                        summary.final_score().home()
                    );
                    info!("Winner: {:?}", summary.winner());
                    return;
                }
            }
        }

        if let Some(game) = advance.game_ref() {
            info!("  Reached the 9th inning!");
            info!("  {}", game.inning_description());
            info!("  Score: Away {} - Home {}", game.score().away(), game.score().home());
        }

        // 9th inning drama
        info!("🎯 9th Inning - Game on the line!");

        // Top 9th: Away team scores 2 runs
        info!("🔝 Top 9th:");
        advance = advance.advance(PitchOutcome::HomeRun);
        if advance.game_ref().is_some() {
            info!("  Batter #1: HOME RUN!");
        } else {
            return;
        }

        advance = advance.advance(PitchOutcome::HomeRun);
        if advance.game_ref().is_some() {
            info!("  Batter #2: ANOTHER HOME RUN!");
        } else {
            return;
        }

        // Need two more outs to complete top 9th
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        if advance.game_ref().is_some() {
            info!("  Batter #3: Out");
        } else {
            return;
        }

        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        if advance.game_ref().is_some() {
            info!("  Batter #4: Out");
        } else {
            return;
        }

        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        if let Some(game) = advance.game_ref() {
            info!("  Batter #5: Out - Top 9th complete!");
            info!("  Score: Away {} - Home {}", game.score().away(), game.score().home());
        } else {
            return;
        }

        // Bottom 9th: Walk-off opportunity
        info!("🔽 Bottom 9th - Walk-off situation!");
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Single));
        let game = advance.game_ref().expect("a single cannot end the game");
        info!("  Batter #1: Single!");
        info!("  Score: Away {} - Home {}", game.score().away(), game.score().home());

        // Home team walk-off home run. This used to log an error because the
        // game kept going; it now ends the moment the run scores.
        advance = advance.advance(PitchOutcome::HomeRun);
        let summary = advance
            .summary_ref()
            .expect("a walk-off home run must end the game immediately");
        info!("  Batter #1: WALK-OFF HOME RUN! GAME OVER! 🎆");
        assert_eq!(summary.winner(), GameWinner::Home);
        assert!(
            summary.final_score().home() > summary.final_score().away(),
            "the walk-off should leave the home team ahead"
        );
    }

    #[test]
    fn a_walk_off_ends_an_extra_inning_without_waiting_for_the_third_out() {
        // Regression: the walk-off check only looked for the ninth, so an extra
        // innings winner sat on the field until the side was retired.
        let mut game = Game::new();
        game.current_inning = InningNumber::Extra(10);
        game.state = GameStatus::Inning(InningHalf::Bottom);
        game.score = GameScore::new().add_away_runs(4).add_home_runs(4);

        let outcome = game.advance(PitchOutcome::HomeRun);

        let summary = outcome
            .summary_ref()
            .expect("the home run should have won the game on the spot");
        assert_eq!(summary.winner(), GameWinner::Home);
        assert_eq!(summary.final_score().home(), 5);
        assert_eq!(summary.innings_played().as_number(), 10);
    }

    #[test]
    fn a_tie_after_nine_goes_to_extras_rather_than_ending() {
        let mut game = Game::new();
        game.current_inning = InningNumber::Ninth;
        game.state = GameStatus::Inning(InningHalf::Bottom);
        game.score = GameScore::new().add_away_runs(2).add_home_runs(2);

        let mut outcome = GameOutcome::InProgress(game);
        for _ in 0..3 {
            outcome = outcome.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        }

        let game = outcome.game_ref().expect("a tie game must continue into extras");
        assert_eq!(game.current_inning(), InningNumber::Extra(10));
    }

    #[test]
    fn the_line_score_records_runs_by_half_inning_and_counts_hits() {
        let mut outcome = GameOutcome::InProgress(Game::new());

        // Top 1st: solo homer, then three outs.
        outcome = outcome.advance(PitchOutcome::HomeRun);
        for _ in 0..3 {
            outcome = outcome.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        }
        // Bottom 1st: a single, stranded by three outs.
        outcome = outcome.advance(PitchOutcome::InPlay(PlayResult::Single));
        for _ in 0..3 {
            outcome = outcome.advance(PitchOutcome::InPlay(PlayResult::Flyout));
        }

        let game = outcome.game_ref().expect("still the second inning");
        let line = game.line_score();

        assert_eq!(line.away_innings(), &[1], "away scored one in the first");
        assert_eq!(line.home_innings(), &[0], "home was shut out in the first");
        assert_eq!(line.away_hits(), 1, "the home run is a hit");
        assert_eq!(line.home_hits(), 1, "the single is a hit");
        assert_eq!(line.columns(), 1);
    }

    #[test]
    fn the_line_score_leaves_the_home_ninth_blank_when_it_is_never_played() {
        // Home team leading after the top of the ninth: they do not bat, so the
        // scoreboard has one fewer column for them.
        let mut game = Game::new();
        game.current_inning = InningNumber::Ninth;
        game.state = GameStatus::Inning(InningHalf::Top);
        game.score = GameScore::new().add_home_runs(3).add_away_runs(1);

        let mut outcome = GameOutcome::InProgress(game);
        for _ in 0..3 {
            outcome = outcome.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        }

        let summary = outcome.summary_ref().expect("home team leads, game is over");
        let line = summary.line_score();
        assert_eq!(line.away_innings().len(), 1, "only the top half we simulated");
        assert!(line.home_innings().is_empty(), "the home ninth was never played");
    }

    #[test]
    fn a_decided_game_stops_accepting_pitches() {
        let mut game = Game::new();
        game.current_inning = InningNumber::Ninth;
        game.state = GameStatus::Inning(InningHalf::Bottom);
        game.score = GameScore::new().add_away_runs(1).add_home_runs(1);

        let outcome = game.advance(PitchOutcome::HomeRun);
        assert!(outcome.is_complete());

        // Further pitches are ignored rather than corrupting the final score.
        let final_score = outcome.summary_ref().expect("game is over").final_score();
        let outcome = outcome.advance(PitchOutcome::HomeRun);
        assert_eq!(
            outcome.summary_ref().expect("still over").final_score(),
            final_score,
            "a completed game must not keep scoring"
        );
    }
}
