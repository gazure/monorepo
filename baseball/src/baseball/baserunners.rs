use std::fmt::Display;

use crate::{
    baseball::{inning::Outs, lineup::BattingPosition},
    HomePlateRuns, Runs,
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
        match self {
            Base::First => Base::Second,
            Base::Second => Base::Third,
            Base::Third => Base::Home,
            Base::Home => Base::Home, // Can't advance past home
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
        match self {
            BaseOutcome::ForceOut => write!(f, "Force Out"),
            BaseOutcome::TagOut => write!(f, "Tag Out"),
            BaseOutcome::Runner(batting_position) => write!(f, "Runner: {batting_position}"),
            BaseOutcome::None => write!(f, "None"),
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
        self.outs().has_outs()
    }

    fn as_basrunner(self) -> Option<BattingPosition> {
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
        self.outs().has_outs()
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayOutcome {
    first: BaseOutcome,
    second: BaseOutcome,
    third: BaseOutcome,
    home: HomeOutcome,
}

impl Display for PlayOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}, {}, {}", self.first, self.second, self.third, self.home)
    }
}

impl PlayOutcome {
    pub fn new(first: BaseOutcome, second: BaseOutcome, third: BaseOutcome, home: HomeOutcome) -> Self {
        PlayOutcome {
            first,
            second,
            third,
            home,
        }
    }

    pub fn groundout() -> Self {
        PlayOutcome {
            first: BaseOutcome::ForceOut,
            second: BaseOutcome::None,
            third: BaseOutcome::None,
            home: HomeOutcome::none(),
        }
    }

    pub fn single(baserunners: BaserunnerState, batter: BattingPosition) -> PlayOutcome {
        PlayOutcome {
            first: BaseOutcome::Runner(batter),
            second: baserunners
                .first()
                .map(BaseOutcome::Runner)
                .unwrap_or(BaseOutcome::None),
            third: baserunners
                .second()
                .map(BaseOutcome::Runner)
                .unwrap_or(BaseOutcome::None),
            home: Self::scored(None, None, baserunners.third(), None),
        }
    }

    pub fn double(baserunners: BaserunnerState, batter: BattingPosition) -> PlayOutcome {
        PlayOutcome {
            first: BaseOutcome::None,
            second: BaseOutcome::Runner(batter),
            third: baserunners
                .first()
                .map(BaseOutcome::Runner)
                .unwrap_or(BaseOutcome::None),
            home: Self::scored(None, baserunners.second(), baserunners.third(), None),
        }
    }

    pub fn triple(baserunners: BaserunnerState, batter: BattingPosition) -> PlayOutcome {
        let home = Self::scored(baserunners.first(), baserunners.second(), baserunners.third(), None);
        PlayOutcome {
            first: BaseOutcome::None,
            second: BaseOutcome::None,
            third: BaseOutcome::Runner(batter),
            home,
        }
    }

    pub fn homerun(baserunners: BaserunnerState, batter: BattingPosition) -> PlayOutcome {
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
        }
    }

    pub fn outs(self) -> Outs {
        self.first().outs() + self.second().outs() + self.third().outs() + self.home.outs()
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
        Self {
            first,
            second: self.second,
            third: self.third,
            home: self.home,
        }
    }

    pub fn with_second(self, second: BaseOutcome) -> Self {
        Self {
            first: self.first,
            second,
            third: self.third,
            home: self.home,
        }
    }

    pub fn with_third(self, third: BaseOutcome) -> Self {
        Self {
            first: self.first,
            second: self.second,
            third,
            home: self.home,
        }
    }

    pub fn with_home(self, home: HomeOutcome) -> Self {
        Self {
            first: self.first,
            second: self.second,
            third: self.third,
            home,
        }
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
            first: self.first.as_basrunner(),
            second: self.second.as_basrunner(),
            third: self.third.as_basrunner(),
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
            BaseOutcome::Runner(BattingPosition::Third).as_basrunner(),
            Some(BattingPosition::Third)
        );
        assert_eq!(BaseOutcome::ForceOut.as_basrunner(), None);
        assert_eq!(BaseOutcome::TagOut.as_basrunner(), None);
        assert_eq!(BaseOutcome::None.as_basrunner(), None);
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
        let groundout = PlayOutcome::groundout();
        assert_eq!(groundout.first(), BaseOutcome::ForceOut);
        assert_eq!(groundout.second(), BaseOutcome::None);
        assert_eq!(groundout.third(), BaseOutcome::None);
        assert_eq!(groundout.home().runs, HomePlateRuns::Zero);
        assert_eq!(groundout.outs(), Outs::One);
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
    fn test_play_outcome_homerun() {
        let baserunners = BaserunnerState::new()
            .set_first(Some(BattingPosition::Second))
            .set_third(Some(BattingPosition::Fourth));

        let homerun = PlayOutcome::homerun(baserunners, BattingPosition::First);

        assert_eq!(homerun.first(), BaseOutcome::None);
        assert_eq!(homerun.second(), BaseOutcome::None);
        assert_eq!(homerun.third(), BaseOutcome::None);
        assert_eq!(homerun.runs_scored(), 3); // Two baserunners + batter
    }

    #[test]
    fn test_play_outcome_with_methods() {
        let outcome = PlayOutcome::groundout();

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
