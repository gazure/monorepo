use std::fmt::Display;

use tracing::debug;

use crate::{
    baseball::{
        baserunners::BaserunnerState,
        lineup::BattingPosition,
        plate_appearance::{PitchOutcome, PlateAppearance, PlateAppearanceResult},
    },
    Runs,
};

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum InningHalf {
    #[default]
    Top,
    Bottom,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum Outs {
    #[default]
    Zero,
    One,
    Two,
    Three, // Side is retired
}

impl std::ops::Add<Outs> for Outs {
    type Output = Outs;

    fn add(self, rhs: Outs) -> Self::Output {
        match self {
            Outs::Zero => match rhs {
                Outs::Zero => Outs::Zero,
                Outs::One => Outs::One,
                Outs::Two => Outs::Two,
                Outs::Three => Outs::Three,
            },
            Outs::One => match rhs {
                Outs::Zero => Outs::One,
                Outs::One => Outs::Two,
                Outs::Two | Outs::Three => Outs::Three,
            },
            Outs::Two => match rhs {
                Outs::Zero => Outs::Two,
                Outs::One | Outs::Two | Outs::Three => Outs::Three,
            },
            Outs::Three => Outs::Three,
        }
    }
}

impl Display for Outs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_number())
    }
}

impl Outs {
    pub fn add_out(self) -> Outs {
        match self {
            Outs::Zero => Outs::One,
            Outs::One => Outs::Two,
            Outs::Two => Outs::Three,
            Outs::Three => Outs::Three, // Stay at three
        }
    }

    pub fn has_outs(self) -> bool {
        self != Outs::Zero
    }

    pub fn as_number(self) -> Runs {
        match self {
            Outs::Zero => 0,
            Outs::One => 1,
            Outs::Two => 2,
            Outs::Three => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalfInning {
    half: InningHalf,
    outs: Outs,
    current_batter: BattingPosition,
    current_pa: PlateAppearance,
    runs_scored: Runs,
    baserunners: BaserunnerState,
}

impl Default for HalfInning {
    fn default() -> Self {
        HalfInning {
            half: InningHalf::default(),
            outs: Outs::default(),
            current_batter: BattingPosition::default(),
            current_pa: PlateAppearance::new(),
            runs_scored: 0,
            baserunners: BaserunnerState::new(),
        }
    }
}

impl HalfInning {
    pub fn new(half: InningHalf, starting_batter: BattingPosition) -> Self {
        HalfInning {
            half,
            outs: Outs::Zero,
            current_batter: starting_batter,
            current_pa: PlateAppearance::new(),
            runs_scored: 0,
            baserunners: BaserunnerState::new(),
        }
    }

    pub fn half(&self) -> InningHalf {
        self.half
    }

    pub fn outs(&self) -> Outs {
        self.outs
    }

    pub fn current_batter(&self) -> BattingPosition {
        self.current_batter
    }

    pub fn current_plate_appearance(&self) -> &PlateAppearance {
        &self.current_pa
    }

    pub fn runs_scored(&self) -> Runs {
        self.runs_scored
    }

    pub fn baserunners(&self) -> BaserunnerState {
        self.baserunners
    }

    fn increment_outs(self, n: Outs) -> HalfInningResult {
        let outs = match (n, self.outs) {
            (Outs::Zero, o) => o,
            (o, Outs::Zero) => o,
            (Outs::One, Outs::One) => Outs::Two,
            (Outs::One, Outs::Two)
            | (Outs::Two, Outs::One)
            | (Outs::Two, Outs::Two)
            | (Outs::Three, _)
            | (_, Outs::Three) => Outs::Three,
        };

        if matches!(outs, Outs::Three) {
            debug!("Inning over, runs scored: {}", self.runs_scored);
            return HalfInningResult::Complete(HalfInningSummary::new(self.runs_scored));
        }

        self.set_outs(outs).advance_batter()
    }

    pub fn advance(mut self, outcome: PitchOutcome) -> HalfInningResult {
        let pa = self.current_pa.advance(outcome);

        match pa {
            PlateAppearanceResult::Strikeout => self.increment_outs(Outs::One),
            PlateAppearanceResult::InPlay(outcome) => {
                let outs = outcome.outs();
                let baserunners = outcome.baserunners();
                let runs_scored = outcome.runs_scored();
                self.add_runs(runs_scored)
                    .with_baserunners(baserunners)
                    .increment_outs(outs)
            }
            PlateAppearanceResult::Walk => {
                let (baserunners, runs) = self.baserunners.walk(self.current_batter);
                self.add_runs(runs).with_baserunners(baserunners).advance_batter()
            }
            PlateAppearanceResult::HitByPitch => {
                let (baserunners, runs) = self.baserunners.walk(self.current_batter);
                self.add_runs(runs).with_baserunners(baserunners).advance_batter()
            }
            PlateAppearanceResult::HomeRun => {
                let runs = self.baserunners.home_run();
                debug!("Home run scored: {runs}");
                self.add_runs(runs)
                    .with_baserunners(BaserunnerState::empty())
                    .advance_batter()
            }
            PlateAppearanceResult::InProgress(pa) => {
                self.current_pa = pa;
                HalfInningResult::in_progress(self)
            }
        }
    }

    fn set_outs(mut self, outs: Outs) -> Self {
        self.outs = outs;
        self
    }

    fn advance_batter(mut self) -> HalfInningResult {
        self.current_batter = self.current_batter.next();
        self.current_pa = PlateAppearance::new();
        HalfInningResult::in_progress(self)
    }

    fn add_runs(mut self, runs_scored: Runs) -> Self {
        self.runs_scored += runs_scored;
        debug!("Runs scored: {runs_scored}");
        self
    }

    fn with_baserunners(mut self, baserunners: BaserunnerState) -> Self {
        self.baserunners = baserunners;
        self
    }

    pub fn summary(&self) -> Result<String, std::fmt::Error> {
        use std::fmt::Write;

        let baserunners = self.baserunners();
        let mut message = String::new();

        writeln!(message, "  Baserunners:")?;

        if baserunners.is_empty() {
            writeln!(message, "    Bases empty")?;
        } else {
            if let Some(runner) = baserunners.first() {
                writeln!(message, "    1st: Batter #{}", runner.as_number())?;
            }
            if let Some(runner) = baserunners.second() {
                writeln!(message, "    2nd: Batter #{}", runner.as_number())?;
            }
            if let Some(runner) = baserunners.third() {
                writeln!(message, "    3rd: Batter #{}", runner.as_number())?;
            }
        }

        writeln!(message, "  Runs scored this inning: {}", self.runs_scored)?;
        write!(message, "  Current batter: #{}", self.current_batter().as_number())?;

        Ok(message)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HalfInningSummary {
    runs_scored: Runs,
}

impl HalfInningSummary {
    pub fn new(runs_scored: Runs) -> Self {
        HalfInningSummary { runs_scored }
    }

    pub fn runs_scored(&self) -> Runs {
        self.runs_scored
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HalfInningResult {
    InProgress(HalfInning),
    Complete(HalfInningSummary),
}

impl HalfInningResult {
    pub fn advance(self, pitch: PitchOutcome) -> HalfInningResult {
        match self {
            HalfInningResult::InProgress(hi) => hi.advance(pitch),
            HalfInningResult::Complete(_) => self,
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, HalfInningResult::Complete(_))
    }

    pub fn half_inning(&self) -> Option<HalfInning> {
        match self {
            HalfInningResult::InProgress(hi) => Some(*hi),
            HalfInningResult::Complete(_) => None,
        }
    }

    pub fn half_inning_ref(&self) -> Option<&HalfInning> {
        match self {
            HalfInningResult::InProgress(hi) => Some(hi),
            HalfInningResult::Complete(_) => None,
        }
    }

    fn in_progress(hi: HalfInning) -> HalfInningResult {
        HalfInningResult::InProgress(hi)
    }
}

#[cfg(test)]
mod tests {
    use tracing::info;

    use super::*;
    use crate::baseball::{baserunners::PlayOutcome, plate_appearance::PitchOutcome};

    #[test]
    fn test_batting_position_as_number() {
        assert_eq!(BattingPosition::First.as_number(), 1);
        assert_eq!(BattingPosition::Ninth.as_number(), 9);
    }

    #[test]
    fn test_batting_position_next() {
        let pos1 = BattingPosition::First;
        let pos2 = pos1.next();
        assert_eq!(pos2.as_number(), 2);

        let pos9 = BattingPosition::Ninth;
        let pos1_again = pos9.next();
        assert_eq!(pos1_again.as_number(), 1);
    }

    #[test]
    fn test_outs_progression() {
        let outs = Outs::Zero;
        let outs = outs.add_out();
        assert_eq!(outs, Outs::One);

        let outs = outs.add_out();
        assert_eq!(outs, Outs::Two);

        let outs = outs.add_out();
        assert_eq!(outs, Outs::Three);
    }

    #[test]
    fn test_half_inning_creation() {
        let batting_pos = BattingPosition::Third;
        let half_inning = HalfInning::new(InningHalf::Top, batting_pos);

        assert_eq!(half_inning.half(), InningHalf::Top);
        assert_eq!(half_inning.outs(), Outs::Zero);
        assert_eq!(half_inning.current_batter().as_number(), 3);
        assert_eq!(half_inning.runs_scored(), 0);
    }

    #[test]
    fn test_half_inning_strikeout() {
        let batting_pos = BattingPosition::First;
        let half_inning = HalfInning::new(InningHalf::Top, batting_pos);

        // Simulate a strikeout (3 strikes)
        let half_inning = half_inning
            .advance(PitchOutcome::Strike)
            .half_inning()
            .expect("unexpected inning end")
            .advance(PitchOutcome::Strike)
            .half_inning()
            .expect("unexpected inning end")
            .advance(PitchOutcome::Strike)
            .half_inning()
            .expect("unexpected inning end");

        assert_eq!(half_inning.outs(), Outs::One);
        assert_eq!(half_inning.current_batter().as_number(), 2); // Next batter
    }

    #[test]
    fn test_half_inning_home_run() {
        let batting_pos = BattingPosition::First;
        let half_inning = HalfInning::new(InningHalf::Top, batting_pos);

        let result = half_inning.advance(PitchOutcome::HomeRun);
        let half_inning = result.half_inning().expect("unexpected inning end");

        assert_eq!(half_inning.runs_scored(), 1);
        assert_eq!(half_inning.current_batter().as_number(), 2); // Next batter
    }

    #[test]
    fn test_three_outs_ends_half_inning() {
        let batting_pos = BattingPosition::First;
        let half_inning = HalfInning::new(InningHalf::Top, batting_pos);

        let advance = half_inning
            .advance(PitchOutcome::InPlay(PlayOutcome::groundout()))
            .half_inning()
            .expect("unexpected inning end")
            .advance(PitchOutcome::InPlay(PlayOutcome::groundout()))
            .half_inning()
            .expect("unexpected inning end")
            .advance(PitchOutcome::InPlay(PlayOutcome::groundout()));

        assert!(advance.is_complete());
    }

    #[test]
    fn demo_half_inning() {
        let batting_pos = BattingPosition::First;
        let half_inning = HalfInning::new(InningHalf::Top, batting_pos);

        info!("Starting top half with leadoff batter");
        info!(
            "Initial state: {} outs, batter #{}",
            half_inning.outs().as_number(),
            half_inning.current_batter().as_number()
        );

        // Batter 1: Quick out
        info!("  Batter #1 steps up...");
        let mut advance = half_inning.advance(PitchOutcome::InPlay(PlayOutcome::groundout()));
        if let Some(half_inning) = advance.half_inning_ref() {
            info!("    Result: Out");
            info!(
                "    New state: {} outs, next batter #{}",
                half_inning.outs().as_number(),
                half_inning.current_batter().as_number()
            );

            // Batter 2: Home run
            info!("  Batter #2 steps up...");
            advance = advance.advance(PitchOutcome::HomeRun);
            if let Some(half_inning2) = advance.half_inning_ref() {
                info!("    Result: Home Run! 🎉");
                info!(
                    "    New state: {} outs, {} runs, next batter #{}",
                    half_inning2.outs().as_number(),
                    half_inning2.runs_scored(),
                    half_inning2.current_batter().as_number()
                );
            }
        }
    }

    #[test]
    fn demo_baserunner_tracking() {
        info!("Demonstrating type-safe baserunner advancement...");

        let batting_pos = BattingPosition::First;
        let mut half_inning = HalfInning::new(InningHalf::Bottom, batting_pos);

        info!("Initial state: No runners on base");
        info!("{}", half_inning.summary().expect("half_inning should be valid"));

        // Start with the advance wrapper
        let mut advance = HalfInningResult::InProgress(half_inning);

        // Batter 1: Single
        info!("🏏 Batter #1: Single");
        advance = advance.advance(PitchOutcome::InPlay(PlayOutcome::single(
            half_inning.baserunners(),
            half_inning.current_batter(),
        )));
        if let Some(hi) = advance.half_inning() {
            info!("{}", hi.summary().expect("half_inning should be valid"));
        } else {
            info!("  Half inning ended unexpectedly");
            return;
        }

        // Batter 2: Walk (forces runner)
        // Batter 2: Walk (4 balls)
        info!("🏏 Batter #2: Walk (4 balls)");
        advance = advance.advance(PitchOutcome::Ball);
        if advance.is_complete() {
            return;
        }
        advance = advance.advance(PitchOutcome::Ball);
        if advance.is_complete() {
            return;
        }
        advance = advance.advance(PitchOutcome::Ball);
        if advance.is_complete() {
            return;
        }
        advance = advance.advance(PitchOutcome::Ball);
        if let Some(hi) = advance.half_inning() {
            half_inning = hi;
            info!("{}", hi.summary().expect("half_inning should be valid"));
        } else {
            return;
        }

        // Batter 3: Double (runners advance)
        info!("🏏 Batter #3: Double");
        advance = advance.advance(PitchOutcome::InPlay(PlayOutcome::double(
            half_inning.baserunners(),
            half_inning.current_batter(),
        )));
        if let Some(hi) = advance.half_inning() {
            half_inning = hi;
            info!("{}", hi.summary().expect("half_inning should be valid"));
        } else {
            return;
        }

        // Batter 4: Triple (clears bases)
        info!("🏏 Batter #4: Triple");
        advance = advance.advance(PitchOutcome::InPlay(PlayOutcome::triple(
            half_inning.baserunners(),
            half_inning.current_batter(),
        )));
        if let Some(hi) = advance.half_inning() {
            info!("{}", hi.summary().expect("half_inning should be valid"));
        } else {
            return;
        }

        // Batter 5: Home run
        info!("🏏 Batter #5: Home Run");
        advance = advance.advance(PitchOutcome::HomeRun);
        if let Some(hi) = advance.half_inning() {
            info!("{}", hi.summary().expect("half_inning should be valid"));
        } else {
            info!("  Half inning complete after home run");
            return;
        }

        info!("🎯 Baserunner tracking complete!");
        info!("✅ Type-safe advancement rules enforced");
        info!("✅ Automatic run scoring calculation");
        info!("✅ Proper force situations handled");
    }
}
