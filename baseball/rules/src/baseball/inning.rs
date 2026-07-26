use std::fmt::Display;

use tracingx::debug;

use super::{
    baserunners::BaserunnerState,
    lineup::BattingPosition,
    plate_appearance::{PitchOutcome, PlateAppearance, PlateAppearanceResult},
    runs::Runs,
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
        use Outs::*;
        match self {
            Zero => match rhs {
                Zero => Zero,
                One => One,
                Two => Two,
                Three => Three,
            },
            One => match rhs {
                Zero => One,
                One => Two,
                Two | Three => Three,
            },
            Two => match rhs {
                Zero => Two,
                One | Two | Three => Three,
            },
            Three => Three,
        }
    }
}

impl Display for Outs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_number())
    }
}

impl Outs {
    pub fn inc(self) -> Outs {
        self + Outs::One
    }

    pub fn is_zero(self) -> bool {
        self == Outs::Zero
    }

    pub fn as_number(self) -> Runs {
        use Outs::*;
        match self {
            Zero => 0,
            One => 1,
            Two => 2,
            Three => 3,
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
        Self::new(InningHalf::default(), BattingPosition::default())
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

    fn increment_outs(self, inc: Outs) -> HalfInningResult {
        let outs = self.outs + inc;

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
            PlateAppearanceResult::InPlay(play) => {
                // Resolved here, against the baserunners this half inning already
                // owns, so no caller can hand us a stale copy of them.
                let outcome = play.resolve(self.baserunners, self.current_batter);
                let outs = outcome.outs();
                let baserunners = outcome.baserunners();

                // Rule 5.08(a): a third out that is a force play, or the batter
                // retired before first, wipes out any run scored on the same play.
                let ends_inning = matches!(self.outs + outs, Outs::Three);
                let runs_scored = if ends_inning && outcome.suppresses_runs_on_third_out() {
                    0
                } else {
                    outcome.runs_scored()
                };

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
                writeln!(message, "    1st: Batter #{}", runner.num())?;
            }
            if let Some(runner) = baserunners.second() {
                writeln!(message, "    2nd: Batter #{}", runner.num())?;
            }
            if let Some(runner) = baserunners.third() {
                writeln!(message, "    3rd: Batter #{}", runner.num())?;
            }
        }

        writeln!(message, "  Runs scored this inning: {}", self.runs_scored)?;
        write!(message, "  Current batter: #{}", self.current_batter().num())?;

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
    use tracingx::info;

    use super::*;
    use crate::baseball::{plate_appearance::PitchOutcome, play::PlayResult};

    #[test]
    fn test_batting_position_as_number() {
        assert_eq!(BattingPosition::First.num(), 1);
        assert_eq!(BattingPosition::Ninth.num(), 9);
    }

    #[test]
    fn test_batting_position_next() {
        let pos1 = BattingPosition::First;
        let pos2 = pos1.next();
        assert_eq!(pos2.num(), 2);

        let pos9 = BattingPosition::Ninth;
        let pos1_again = pos9.next();
        assert_eq!(pos1_again.num(), 1);
    }

    #[test]
    fn test_outs_progression() {
        let outs = Outs::Zero;
        let outs = outs.inc();
        assert_eq!(outs, Outs::One);

        let outs = outs.inc();
        assert_eq!(outs, Outs::Two);

        let outs = outs.inc();
        assert_eq!(outs, Outs::Three);
    }

    #[test]
    fn test_half_inning_creation() {
        let batting_pos = BattingPosition::Third;
        let half_inning = HalfInning::new(InningHalf::Top, batting_pos);

        assert_eq!(half_inning.half(), InningHalf::Top);
        assert_eq!(half_inning.outs(), Outs::Zero);
        assert_eq!(half_inning.current_batter().num(), 3);
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
        assert_eq!(half_inning.current_batter().num(), 2); // Next batter
    }

    #[test]
    fn test_half_inning_home_run() {
        let batting_pos = BattingPosition::First;
        let half_inning = HalfInning::new(InningHalf::Top, batting_pos);

        let result = half_inning.advance(PitchOutcome::HomeRun);
        let half_inning = result.half_inning().expect("unexpected inning end");

        assert_eq!(half_inning.runs_scored(), 1);
        assert_eq!(half_inning.current_batter().num(), 2); // Next batter
    }

    #[test]
    fn test_three_outs_ends_half_inning() {
        let batting_pos = BattingPosition::First;
        let half_inning = HalfInning::new(InningHalf::Top, batting_pos);

        let advance = half_inning
            .advance(PitchOutcome::InPlay(PlayResult::Groundout))
            .half_inning()
            .expect("unexpected inning end")
            .advance(PitchOutcome::InPlay(PlayResult::Groundout))
            .half_inning()
            .expect("unexpected inning end")
            .advance(PitchOutcome::InPlay(PlayResult::Groundout));

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
            half_inning.current_batter().num()
        );

        // Batter 1: Quick out
        info!("  Batter #1 steps up...");
        let mut advance = half_inning.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        if let Some(half_inning) = advance.half_inning_ref() {
            info!("    Result: Out");
            info!(
                "    New state: {} outs, next batter #{}",
                half_inning.outs().as_number(),
                half_inning.current_batter().num()
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
                    half_inning2.current_batter().num()
                );
            }
        }
    }

    /// Walks a full rally through the half inning and checks the bookkeeping at
    /// every step. Replaces a log-only demo that asserted nothing.
    #[test]
    fn a_rally_moves_every_runner_and_banks_every_run() {
        let mut advance = HalfInningResult::InProgress(HalfInning::new(InningHalf::Bottom, BattingPosition::First));

        let state = |advance: &HalfInningResult| advance.half_inning().expect("inning should still be live");

        // #1 singles.
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Single));
        let hi = state(&advance);
        assert_eq!(hi.baserunners().first(), Some(BattingPosition::First));
        assert_eq!(hi.current_batter(), BattingPosition::Second);

        // #2 walks, forcing the leadoff runner to second.
        for _ in 0..4 {
            advance = advance.advance(PitchOutcome::Ball);
        }
        let hi = state(&advance);
        assert_eq!(hi.baserunners().first(), Some(BattingPosition::Second));
        assert_eq!(hi.baserunners().second(), Some(BattingPosition::First));
        assert_eq!(hi.runs_scored(), 0);

        // #3 doubles: the runner from second scores, the runner from first
        // reaches third, and the batter is standing on second.
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Double));
        let hi = state(&advance);
        assert_eq!(hi.runs_scored(), 1);
        assert_eq!(hi.baserunners().second(), Some(BattingPosition::Third));
        assert_eq!(hi.baserunners().third(), Some(BattingPosition::Second));
        assert_eq!(hi.baserunners().first(), None);

        // #4 triples, clearing both runners ahead.
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Triple));
        let hi = state(&advance);
        assert_eq!(hi.runs_scored(), 3);
        assert_eq!(hi.baserunners().third(), Some(BattingPosition::Fourth));
        assert_eq!(hi.baserunners().runner_count(), 1);

        // #5 homers, scoring the runner from third plus themselves.
        advance = advance.advance(PitchOutcome::HomeRun);
        let hi = state(&advance);
        assert_eq!(hi.runs_scored(), 5);
        assert!(hi.baserunners().is_empty(), "a home run clears the bases");
        assert_eq!(hi.outs(), Outs::Zero, "the rally never made an out");
        assert_eq!(hi.current_batter(), BattingPosition::Sixth);
    }

    #[test]
    fn a_groundout_leaves_the_runners_alone_but_still_records_the_out() {
        let mut advance = HalfInningResult::InProgress(HalfInning::new(InningHalf::Top, BattingPosition::First));

        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Single));
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));

        let hi = advance.half_inning().expect("one out should not end the inning");
        assert_eq!(hi.outs(), Outs::One);
        assert_eq!(
            hi.baserunners().first(),
            Some(BattingPosition::First),
            "the runner should still be standing on first"
        );
    }

    #[test]
    fn a_force_out_for_the_third_out_wipes_out_the_run_it_would_have_scored() {
        // Two outs, runner on third, then a ground ball that retires the batter.
        // Rule 5.08(a): the run does not count.
        let mut advance = HalfInningResult::InProgress(HalfInning::new(InningHalf::Top, BattingPosition::First));
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Triple));
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));
        advance = advance.advance(PitchOutcome::InPlay(PlayResult::Groundout));

        let hi = advance.half_inning().expect("two outs should not end the inning");
        assert_eq!(hi.outs(), Outs::Two);
        assert_eq!(hi.runs_scored(), 0);

        // Third out: a sacrifice fly would normally plate the runner from third.
        let advance = advance.advance(PitchOutcome::InPlay(PlayResult::SacrificeFly));
        let summary = match advance {
            HalfInningResult::Complete(summary) => summary,
            HalfInningResult::InProgress(_) => panic!("three outs should have ended the inning"),
        };
        assert_eq!(
            summary.runs_scored(),
            0,
            "the batter was retired for the third out, so the run cannot count"
        );
    }
}
