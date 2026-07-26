use std::fmt::Display;

use super::{
    inning::Outs,
    lineup::BattingPosition,
    runs::{HomePlateRuns, Runs},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Base {
    First,
    Second,
    Third,
    Home,
}

impl Base {
    pub fn next(self) -> Base {
        use Base::*;
        match self {
            First => Second,
            Second => Third,
            Third => Home,
            Home => Home, // Can't advance past home
        }
    }

    pub fn advance_by(self, bases: u8) -> Base {
        let mut current = self;
        for _ in 0..bases {
            if current == Base::Home {
                break;
            }
            current = current.next();
        }
        current
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseOutcome {
    ForceOut,
    TagOut,
    Runner(BattingPosition),
    None,
}

impl Display for BaseOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use BaseOutcome::*;
        match self {
            ForceOut => write!(f, "Force Out"),
            TagOut => write!(f, "Tag Out"),
            Runner(batting_position) => write!(f, "Runner: {batting_position}"),
            None => write!(f, "None"),
        }
    }
}

impl BaseOutcome {
    pub fn outs(&self) -> Outs {
        match self {
            BaseOutcome::ForceOut | BaseOutcome::TagOut => Outs::One,
            _ => Outs::Zero,
        }
    }

    pub fn is_out(&self) -> bool {
        !self.outs().is_zero()
    }

    fn as_baserunner(self) -> Option<BattingPosition> {
        match self {
            BaseOutcome::Runner(batting_position) => Some(batting_position),
            _ => None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct HomeOutcome {
    pub runs: HomePlateRuns,
    pub outs: Outs,
}

impl Display for HomeOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} runs, {} outs", self.runs, self.outs)
    }
}

impl HomeOutcome {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn outs(self) -> Outs {
        self.outs
    }

    pub fn is_out(self) -> bool {
        !self.outs().is_zero()
    }

    fn runs_scored(self) -> Runs {
        self.runs.to_runs()
    }

    pub fn with_runs(self, runs: HomePlateRuns) -> Self {
        HomeOutcome { runs, outs: self.outs }
    }

    pub fn with_outs(self, outs: Outs) -> Self {
        HomeOutcome { runs: self.runs, outs }
    }

    pub fn none() -> Self {
        HomeOutcome {
            runs: HomePlateRuns::Zero,
            outs: Outs::Zero,
        }
    }
}

/// The end state of a single ball in play.
///
/// The three base fields answer "who is standing here when the dust settles",
/// which is exactly what [`PlayOutcome::baserunners`] reads back out. An out
/// recorded *on the batter* therefore cannot live in a base slot — a groundout
/// with a runner on first has to say both "the batter was retired" and "the
/// runner is still on first". That is what `batter_out` is for; without it the
/// runner would be overwritten by the out marker and silently vanish.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayOutcome {
    first: BaseOutcome,
    second: BaseOutcome,
    third: BaseOutcome,
    home: HomeOutcome,
    /// The batter-runner was retired before reaching first.
    batter_out: bool,
}

impl Display for PlayOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}, {}, {}", self.first, self.second, self.third, self.home)?;
        if self.batter_out {
            write!(f, ", batter out")?;
        }
        Ok(())
    }
}

impl PlayOutcome {
    pub fn new(first: BaseOutcome, second: BaseOutcome, third: BaseOutcome, home: HomeOutcome) -> Self {
        PlayOutcome {
            first,
            second,
            third,
            home,
            batter_out: false,
        }
    }

    pub fn with_batter_out(self, batter_out: bool) -> Self {
        Self { batter_out, ..self }
    }

    pub fn batter_out(self) -> bool {
        self.batter_out
    }

    /// Everyone already on base holds where they are. The starting point for
    /// every out that does not force a runner.
    fn runners_hold(baserunners: BaserunnerState) -> Self {
        PlayOutcome {
            first: Self::occupant(baserunners.first()),
            second: Self::occupant(baserunners.second()),
            third: Self::occupant(baserunners.third()),
            home: HomeOutcome::none(),
            batter_out: false,
        }
    }

    fn occupant(runner: Option<BattingPosition>) -> BaseOutcome {
        runner.map_or(BaseOutcome::None, BaseOutcome::Runner)
    }

    /// Batter retired on the ground; the runners stay put.
    pub fn groundout(baserunners: BaserunnerState) -> Self {
        Self::runners_hold(baserunners).with_batter_out(true)
    }

    /// Batter retired in the air; the runners stay put.
    pub fn flyout(baserunners: BaserunnerState) -> Self {
        Self::runners_hold(baserunners).with_batter_out(true)
    }

    /// A caught fly deep enough to score the runner from third. With nobody on
    /// third there is nothing to sacrifice, so it is an ordinary flyout.
    pub fn sacrifice_fly(baserunners: BaserunnerState) -> Self {
        if baserunners.third().is_none() {
            return Self::flyout(baserunners);
        }
        Self {
            first: Self::occupant(baserunners.first()),
            second: Self::occupant(baserunners.second()),
            third: BaseOutcome::None,
            home: Self::scored(None, None, baserunners.third(), None),
            batter_out: true,
        }
    }

    /// Where third base and the plate end up when the runner from first is
    /// retired at second. The out claims the second-base slot, so a runner
    /// standing there must be pushed to third, which in turn pushes any runner on
    /// third across the plate. With second empty, nobody is displaced and the
    /// runner on third simply holds.
    fn force_chain_past_second(baserunners: BaserunnerState) -> (BaseOutcome, HomeOutcome) {
        if baserunners.second().is_some() {
            (
                Self::occupant(baserunners.second()),
                Self::scored(None, None, baserunners.third(), None),
            )
        } else {
            (Self::occupant(baserunners.third()), HomeOutcome::none())
        }
    }

    /// The lead forced runner is retired and the batter reaches first.
    pub fn fielders_choice(baserunners: BaserunnerState, batter: BattingPosition) -> Self {
        // Only a runner on first is forced by the batter alone, so that is the
        // runner the defence can retire without a tag.
        if baserunners.first().is_none() {
            // Nobody forced: this is just an infield single in disguise.
            return Self::single(baserunners, batter);
        }
        let (third, home) = Self::force_chain_past_second(baserunners);
        Self {
            first: BaseOutcome::Runner(batter),
            second: BaseOutcome::ForceOut,
            third,
            home,
            batter_out: false,
        }
    }

    /// Runner on first forced at second and the batter retired at first. Needs a
    /// runner on first to turn; without one it degrades to a plain groundout.
    pub fn double_play(baserunners: BaserunnerState) -> Self {
        if baserunners.first().is_none() {
            return Self::groundout(baserunners);
        }
        let (third, home) = Self::force_chain_past_second(baserunners);
        Self {
            first: BaseOutcome::None,
            second: BaseOutcome::ForceOut,
            third,
            home,
            batter_out: true,
        }
    }

    pub fn single(baserunners: BaserunnerState, batter: BattingPosition) -> PlayOutcome {
        PlayOutcome {
            first: BaseOutcome::Runner(batter),
            second: Self::occupant(baserunners.first()),
            third: Self::occupant(baserunners.second()),
            home: Self::scored(None, None, baserunners.third(), None),
            batter_out: false,
        }
    }

    pub fn double(baserunners: BaserunnerState, batter: BattingPosition) -> PlayOutcome {
        PlayOutcome {
            first: BaseOutcome::None,
            second: BaseOutcome::Runner(batter),
            third: Self::occupant(baserunners.first()),
            home: Self::scored(None, baserunners.second(), baserunners.third(), None),
            batter_out: false,
        }
    }

    pub fn triple(baserunners: BaserunnerState, batter: BattingPosition) -> PlayOutcome {
        let home = Self::scored(baserunners.first(), baserunners.second(), baserunners.third(), None);
        PlayOutcome {
            first: BaseOutcome::None,
            second: BaseOutcome::None,
            third: BaseOutcome::Runner(batter),
            home,
            batter_out: false,
        }
    }

    pub fn home_run(baserunners: BaserunnerState, batter: BattingPosition) -> PlayOutcome {
        PlayOutcome {
            first: BaseOutcome::None,
            second: BaseOutcome::None,
            third: BaseOutcome::None,
            home: Self::scored(
                baserunners.first(),
                baserunners.second(),
                baserunners.third(),
                Some(batter),
            ),
            batter_out: false,
        }
    }

    pub fn outs(self) -> Outs {
        let batter = if self.batter_out { Outs::One } else { Outs::Zero };
        self.first().outs() + self.second().outs() + self.third().outs() + self.home.outs() + batter
    }

    /// Whether a third out produced by this play wipes out its own runs.
    ///
    /// Rule 5.08(a): no run scores if the third out is a force out, or if the
    /// batter-runner is retired before reaching first. A tag out does not
    /// suppress a run that had already crossed the plate.
    pub fn suppresses_runs_on_third_out(self) -> bool {
        self.batter_out
            || matches!(self.first, BaseOutcome::ForceOut)
            || matches!(self.second, BaseOutcome::ForceOut)
            || matches!(self.third, BaseOutcome::ForceOut)
    }

    pub fn first(self) -> BaseOutcome {
        self.first
    }

    pub fn second(self) -> BaseOutcome {
        self.second
    }

    pub fn third(self) -> BaseOutcome {
        self.third
    }

    pub fn home(self) -> HomeOutcome {
        self.home
    }

    pub fn with_first(self, first: BaseOutcome) -> Self {
        Self { first, ..self }
    }

    pub fn with_second(self, second: BaseOutcome) -> Self {
        Self { second, ..self }
    }

    pub fn with_third(self, third: BaseOutcome) -> Self {
        Self { third, ..self }
    }

    pub fn with_home(self, home: HomeOutcome) -> Self {
        Self { home, ..self }
    }

    fn scored(
        first: Option<BattingPosition>,
        second: Option<BattingPosition>,
        third: Option<BattingPosition>,
        batter: Option<BattingPosition>,
    ) -> HomeOutcome {
        let runs: HomePlateRuns = match (first, second, third, batter) {
            (None, None, None, None) => HomePlateRuns::Zero,
            (None, None, None, Some(_)) => HomePlateRuns::One,
            (None, None, Some(_), None) => HomePlateRuns::One,
            (None, None, Some(_), Some(_)) => HomePlateRuns::Two,
            (None, Some(_), None, None) => HomePlateRuns::One,
            (None, Some(_), None, Some(_)) => HomePlateRuns::Two,
            (None, Some(_), Some(_), None) => HomePlateRuns::Two,
            (None, Some(_), Some(_), Some(_)) => HomePlateRuns::Three,
            (Some(_), None, None, None) => HomePlateRuns::One,
            (Some(_), None, None, Some(_)) => HomePlateRuns::Two,
            (Some(_), None, Some(_), None) => HomePlateRuns::Two,
            (Some(_), None, Some(_), Some(_)) => HomePlateRuns::Three,
            (Some(_), Some(_), None, None) => HomePlateRuns::Two,
            (Some(_), Some(_), None, Some(_)) => HomePlateRuns::Three,
            (Some(_), Some(_), Some(_), None) => HomePlateRuns::Three,
            (Some(_), Some(_), Some(_), Some(_)) => HomePlateRuns::Four,
        };

        HomeOutcome::default().with_runs(runs)
    }

    pub fn baserunners(self) -> BaserunnerState {
        BaserunnerState {
            first: self.first.as_baserunner(),
            second: self.second.as_baserunner(),
            third: self.third.as_baserunner(),
        }
    }

    pub fn runs_scored(self) -> Runs {
        self.home.runs_scored()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaserunnerState {
    first: Option<BattingPosition>,
    second: Option<BattingPosition>,
    third: Option<BattingPosition>,
}

impl BaserunnerState {
    pub fn new() -> Self {
        BaserunnerState {
            first: None,
            second: None,
            third: None,
        }
    }

    pub fn empty() -> Self {
        Self::new()
    }

    pub fn is_empty(&self) -> bool {
        self.first.is_none() && self.second.is_none() && self.third.is_none()
    }

    pub fn first(&self) -> Option<BattingPosition> {
        self.first
    }

    pub fn second(&self) -> Option<BattingPosition> {
        self.second
    }

    pub fn third(&self) -> Option<BattingPosition> {
        self.third
    }

    pub fn set_first(mut self, runner: Option<BattingPosition>) -> Self {
        self.first = runner;
        self
    }

    pub fn set_second(mut self, runner: Option<BattingPosition>) -> Self {
        self.second = runner;
        self
    }

    pub fn set_third(mut self, runner: Option<BattingPosition>) -> Self {
        self.third = runner;
        self
    }

    pub fn runner_count(&self) -> u8 {
        let mut count = 0;
        if self.first.is_some() {
            count += 1;
        }
        if self.second.is_some() {
            count += 1;
        }
        if self.third.is_some() {
            count += 1;
        }
        count
    }

    pub fn has_runner_on(&self, base: Base) -> bool {
        match base {
            Base::First => self.first.is_some(),
            Base::Second => self.second.is_some(),
            Base::Third => self.third.is_some(),
            Base::Home => false, // No one stays on home
        }
    }

    pub fn walk(&self, batter: BattingPosition) -> (BaserunnerState, Runs) {
        let mut new_state = BaserunnerState::new().set_first(Some(batter));
        let mut runs_scored = Runs::default();

        if let Some(runner) = self.first {
            new_state = new_state.set_second(Some(runner));
        }
        if let Some(runner) = self.second {
            new_state = new_state.set_third(Some(runner));
        }
        if self.third.is_some() {
            runs_scored += 1;
        }

        (new_state, runs_scored)
    }

    pub fn home_run(&self) -> Runs {
        let mut runs = 1;
        if self.first.is_some() {
            runs += 1;
        }
        if self.second.is_some() {
            runs += 1;
        }
        if self.third.is_some() {
            runs += 1;
        }
        runs
    }
}

impl Default for BaserunnerState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_base_advancement() {
        assert_eq!(Base::First.next(), Base::Second);
        assert_eq!(Base::Second.next(), Base::Third);
        assert_eq!(Base::Third.next(), Base::Home);
        assert_eq!(Base::Home.next(), Base::Home);
    }

    #[test]
    fn test_base_advance_by() {
        assert_eq!(Base::First.advance_by(2), Base::Third);
        assert_eq!(Base::Second.advance_by(2), Base::Home);
        assert_eq!(Base::Third.advance_by(5), Base::Home); // Can't go past home
    }

    #[test]
    fn test_baserunner_state_creation() {
        let state = BaserunnerState::new();
        assert!(state.is_empty());
        assert_eq!(state.runner_count(), 0);
        assert!(!state.has_runner_on(Base::First));
    }

    #[test]
    fn test_baserunner_state_with_runners() {
        let state = BaserunnerState::new()
            .set_first(Some(BattingPosition::First))
            .set_third(Some(BattingPosition::Third));

        assert!(!state.is_empty());
        assert_eq!(state.runner_count(), 2);
        assert!(state.has_runner_on(Base::First));
        assert!(!state.has_runner_on(Base::Second));
        assert!(state.has_runner_on(Base::Third));
    }

    #[test]
    fn test_home_outcome_creation() {
        let outcome = HomeOutcome::new().with_runs(HomePlateRuns::One).with_outs(Outs::Two);
        assert_eq!(outcome.runs, HomePlateRuns::One);
        assert_eq!(outcome.outs(), Outs::Two);
        assert!(outcome.is_out());

        let no_runs = HomeOutcome::new().with_outs(Outs::One);
        assert_eq!(no_runs.runs, HomePlateRuns::Zero);
        assert_eq!(no_runs.outs(), Outs::One);

        let no_outs = HomeOutcome::new().with_outs(Outs::Zero);
        assert_eq!(no_outs.runs, HomePlateRuns::Zero);
        assert_eq!(no_outs.outs(), Outs::Zero);
        assert!(!no_outs.is_out());

        let none = HomeOutcome::none();
        assert_eq!(none.runs, HomePlateRuns::Zero);
        assert_eq!(none.outs(), Outs::Zero);
        assert!(!none.is_out());
    }

    #[test]
    fn test_base_outcome_outs() {
        assert_eq!(BaseOutcome::ForceOut.outs(), Outs::One);
        assert_eq!(BaseOutcome::TagOut.outs(), Outs::One);
        assert_eq!(BaseOutcome::Runner(BattingPosition::First).outs(), Outs::Zero);
        assert_eq!(BaseOutcome::None.outs(), Outs::Zero);

        assert!(BaseOutcome::ForceOut.is_out());
        assert!(BaseOutcome::TagOut.is_out());
        assert!(!BaseOutcome::Runner(BattingPosition::First).is_out());
        assert!(!BaseOutcome::None.is_out());
    }

    #[test]
    fn test_base_outcome_as_baserunner() {
        assert_eq!(
            BaseOutcome::Runner(BattingPosition::Third).as_baserunner(),
            Some(BattingPosition::Third)
        );
        assert_eq!(BaseOutcome::ForceOut.as_baserunner(), None);
        assert_eq!(BaseOutcome::TagOut.as_baserunner(), None);
        assert_eq!(BaseOutcome::None.as_baserunner(), None);
    }

    #[test]
    fn test_play_outcome_creation() {
        let outcome = PlayOutcome::new(
            BaseOutcome::Runner(BattingPosition::First),
            BaseOutcome::None,
            BaseOutcome::TagOut,
            HomeOutcome::new().with_runs(HomePlateRuns::One),
        );

        assert_eq!(outcome.first(), BaseOutcome::Runner(BattingPosition::First));
        assert_eq!(outcome.second(), BaseOutcome::None);
        assert_eq!(outcome.third(), BaseOutcome::TagOut);
        assert_eq!(outcome.home().runs, HomePlateRuns::One);
        assert_eq!(outcome.outs(), Outs::One); // Only third base has an out
        assert_eq!(outcome.runs_scored(), 1);
    }

    #[test]
    fn test_play_outcome_groundout() {
        let groundout = PlayOutcome::groundout(BaserunnerState::empty());
        // The out is recorded against the batter, not by parking a marker in a
        // base slot, so every base reads back as unoccupied.
        assert_eq!(groundout.first(), BaseOutcome::None);
        assert_eq!(groundout.second(), BaseOutcome::None);
        assert_eq!(groundout.third(), BaseOutcome::None);
        assert!(groundout.batter_out());
        assert_eq!(groundout.home().runs, HomePlateRuns::Zero);
        assert_eq!(groundout.outs(), Outs::One);
    }

    #[test]
    fn a_groundout_does_not_erase_the_runners_it_found_on_base() {
        // Regression: the old constant-valued `groundout()` hardcoded every base
        // to `None`, so any ground ball wiped the bases clean.
        let loaded = BaserunnerState::new()
            .set_first(Some(BattingPosition::First))
            .set_second(Some(BattingPosition::Second))
            .set_third(Some(BattingPosition::Third));

        let after = PlayOutcome::groundout(loaded).baserunners();

        assert_eq!(after.runner_count(), 3, "a groundout must not clear the bases");
        assert_eq!(after.first(), Some(BattingPosition::First));
        assert_eq!(after.second(), Some(BattingPosition::Second));
        assert_eq!(after.third(), Some(BattingPosition::Third));
    }

    #[test]
    fn test_play_outcome_single() {
        let baserunners = BaserunnerState::new()
            .set_first(Some(BattingPosition::Second))
            .set_third(Some(BattingPosition::Fourth));

        let single = PlayOutcome::single(baserunners, BattingPosition::First);

        assert_eq!(single.first(), BaseOutcome::Runner(BattingPosition::First));
        assert_eq!(single.second(), BaseOutcome::Runner(BattingPosition::Second));
        assert_eq!(single.third(), BaseOutcome::None);
        assert_eq!(single.runs_scored(), 1); // Runner from third scores
    }

    #[test]
    fn test_play_outcome_double() {
        let baserunners = BaserunnerState::new()
            .set_first(Some(BattingPosition::Second))
            .set_second(Some(BattingPosition::Third));

        let double = PlayOutcome::double(baserunners, BattingPosition::First);

        assert_eq!(double.first(), BaseOutcome::None);
        assert_eq!(double.second(), BaseOutcome::Runner(BattingPosition::First));
        assert_eq!(double.third(), BaseOutcome::Runner(BattingPosition::Second));
        assert_eq!(double.runs_scored(), 1); // Runner from second scores
    }

    #[test]
    fn test_play_outcome_triple() {
        let baserunners = BaserunnerState::new()
            .set_first(Some(BattingPosition::Second))
            .set_second(Some(BattingPosition::Third))
            .set_third(Some(BattingPosition::Fourth));

        let triple = PlayOutcome::triple(baserunners, BattingPosition::First);

        assert_eq!(triple.first(), BaseOutcome::None);
        assert_eq!(triple.second(), BaseOutcome::None);
        assert_eq!(triple.third(), BaseOutcome::Runner(BattingPosition::First));
        assert_eq!(triple.runs_scored(), 3); // All baserunners score
    }

    #[test]
    fn test_play_outcome_home_run() {
        let baserunners = BaserunnerState::new()
            .set_first(Some(BattingPosition::Second))
            .set_third(Some(BattingPosition::Fourth));

        let home_run = PlayOutcome::home_run(baserunners, BattingPosition::First);

        assert_eq!(home_run.first(), BaseOutcome::None);
        assert_eq!(home_run.second(), BaseOutcome::None);
        assert_eq!(home_run.third(), BaseOutcome::None);
        assert_eq!(home_run.runs_scored(), 3); // Two baserunners + batter
    }

    #[test]
    fn test_play_outcome_with_methods() {
        let outcome = PlayOutcome::new(
            BaseOutcome::None,
            BaseOutcome::None,
            BaseOutcome::None,
            HomeOutcome::none(),
        );

        let modified = outcome
            .with_first(BaseOutcome::Runner(BattingPosition::First))
            .with_second(BaseOutcome::TagOut)
            .with_home(HomeOutcome::new().with_runs(HomePlateRuns::Two));

        assert_eq!(modified.first(), BaseOutcome::Runner(BattingPosition::First));
        assert_eq!(modified.second(), BaseOutcome::TagOut);
        assert_eq!(modified.third(), BaseOutcome::None);
        assert_eq!(modified.runs_scored(), 2);
        assert_eq!(modified.outs(), Outs::One); // TagOut on second
    }

    #[test]
    fn the_builders_carry_the_batters_out_along_with_them() {
        let modified = PlayOutcome::groundout(BaserunnerState::empty()).with_third(BaseOutcome::TagOut);

        assert!(modified.batter_out(), "with_third dropped the batter's out");
        assert_eq!(modified.outs(), Outs::Two);
    }

    #[test]
    fn test_play_outcome_baserunners() {
        let outcome = PlayOutcome::new(
            BaseOutcome::Runner(BattingPosition::First),
            BaseOutcome::None,
            BaseOutcome::Runner(BattingPosition::Third),
            HomeOutcome::none(),
        );

        let baserunners = outcome.baserunners();
        assert_eq!(baserunners.first(), Some(BattingPosition::First));
        assert_eq!(baserunners.second(), None);
        assert_eq!(baserunners.third(), Some(BattingPosition::Third));
    }
}
