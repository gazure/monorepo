use std::fmt::Display;

use super::baserunners::PlayOutcome;

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum Balls {
    #[default]
    Zero,
    One,
    Two,
    Three,
}

impl Balls {
    pub fn as_number(&self) -> u8 {
        match self {
            Balls::Zero => 0,
            Balls::One => 1,
            Balls::Two => 2,
            Balls::Three => 3,
        }
    }
}

impl std::fmt::Display for Balls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_number())
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum Strikes {
    #[default]
    Zero,
    One,
    Two,
}

impl Strikes {
    pub fn as_number(&self) -> u8 {
        match self {
            Strikes::Zero => 0,
            Strikes::One => 1,
            Strikes::Two => 2,
        }
    }
}

impl std::fmt::Display for Strikes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_number())
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Count {
    balls: Balls,
    strikes: Strikes,
}

impl std::fmt::Display for Count {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.balls, self.strikes)
    }
}

impl Count {
    #[cfg(test)]
    pub fn new(balls: Balls, strikes: Strikes) -> Self {
        Count { balls, strikes }
    }

    pub fn balls(&self) -> Balls {
        self.balls
    }

    pub fn strikes(&self) -> Strikes {
        self.strikes
    }

    pub fn with_balls(self, balls: Balls) -> Self {
        Count {
            balls,
            strikes: self.strikes,
        }
    }

    pub fn with_strikes(self, strikes: Strikes) -> Self {
        Count {
            balls: self.balls,
            strikes,
        }
    }

    pub fn advance(self, outcome: PitchOutcome) -> CountResult {
        match outcome {
            PitchOutcome::Ball => self.advance_ball(),
            PitchOutcome::Strike => self.advance_strike(),
            PitchOutcome::Foul => self.advance_foul(),
            _ => self.into(),
        }
    }

    fn advance_ball(self) -> CountResult {
        match self.balls {
            Balls::Zero => self.with_balls(Balls::One).into(),
            Balls::One => self.with_balls(Balls::Two).into(),
            Balls::Two => self.with_balls(Balls::Three).into(),
            Balls::Three => CountResult::Walk,
        }
    }

    fn advance_strike(self) -> CountResult {
        match self.strikes {
            Strikes::Zero => self.with_strikes(Strikes::One).into(),
            Strikes::One => self.with_strikes(Strikes::Two).into(),
            Strikes::Two => CountResult::Strikeout,
        }
    }

    fn advance_foul(self) -> CountResult {
        match self.strikes {
            Strikes::Zero => self.with_strikes(Strikes::One),
            Strikes::One => self.with_strikes(Strikes::Two),
            Strikes::Two => self,
        }
        .into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CountResult {
    InProgress(Count),
    Strikeout,
    Walk,
}

impl From<Count> for CountResult {
    fn from(count: Count) -> Self {
        CountResult::InProgress(count)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PitchOutcome {
    Ball,
    Strike,
    Foul,
    InPlay(PlayOutcome),
    HomeRun,
    HitByPitch,
}

impl Display for PitchOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PitchOutcome::Ball => write!(f, "Ball"),
            PitchOutcome::Strike => write!(f, "Strike"),
            PitchOutcome::Foul => write!(f, "Foul"),
            PitchOutcome::InPlay(play_outcome) => write!(f, "InPlay({play_outcome})"),
            PitchOutcome::HomeRun => write!(f, "HomeRun"),
            PitchOutcome::HitByPitch => write!(f, "HitByPitch"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlateAppearance {
    count: Count,
}

impl PlateAppearance {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_count(count: Count) -> Self {
        Self { count }
    }

    pub fn advance(self, outcome: PitchOutcome) -> PlateAppearanceResult {
        match outcome {
            PitchOutcome::Ball | PitchOutcome::Strike | PitchOutcome::Foul => {
                let count_advance = self.count.advance(outcome);

                match count_advance {
                    CountResult::InProgress(count) => {
                        PlateAppearanceResult::InProgress(PlateAppearance::with_count(count))
                    }
                    CountResult::Strikeout => PlateAppearanceResult::Strikeout,
                    CountResult::Walk => PlateAppearanceResult::Walk,
                }
            }
            PitchOutcome::InPlay(outcome) => PlateAppearanceResult::InPlay(outcome),
            PitchOutcome::HomeRun => PlateAppearanceResult::HomeRun,
            PitchOutcome::HitByPitch => PlateAppearanceResult::HitByPitch,
        }
    }

    pub fn count(&self) -> Count {
        self.count
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlateAppearanceResult {
    InProgress(PlateAppearance),
    InPlay(PlayOutcome),
    Walk,
    Strikeout,
    HitByPitch,
    HomeRun,
}

impl PlateAppearanceResult {
    pub fn advance(self, outcome: PitchOutcome) -> PlateAppearanceResult {
        match self {
            PlateAppearanceResult::InProgress(pa) => pa.advance(outcome),
            _ => self, // Already complete, ignore the pitch
        }
    }

    pub fn is_in_progress(self) -> bool {
        !self.is_complete()
    }

    pub fn is_complete(self) -> bool {
        match self {
            PlateAppearanceResult::InProgress(_) => false,
            PlateAppearanceResult::Walk => true,
            PlateAppearanceResult::Strikeout => true,
            PlateAppearanceResult::InPlay(_) => true,
            PlateAppearanceResult::HitByPitch => true,
            PlateAppearanceResult::HomeRun => true,
        }
    }

    pub fn plate_appearance(self) -> Option<PlateAppearance> {
        match self {
            PlateAppearanceResult::InProgress(pa) => Some(pa),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        super::{
            baserunners::{BaseOutcome, BaserunnerState},
            lineup::BattingPosition,
        },
        *,
    };

    #[test]
    fn test_count_new() {
        let count = Count::new(Balls::One, Strikes::Two);
        assert_eq!(count.balls(), Balls::One);
        assert_eq!(count.strikes(), Strikes::Two);
    }

    #[test]
    fn test_count_default() {
        let count = Count::default();
        assert_eq!(count.balls(), Balls::Zero);
        assert_eq!(count.strikes(), Strikes::Zero);
    }

    #[test]
    fn test_plate_appearance_new() {
        let pa = PlateAppearance::new();
        assert_eq!(pa.count(), Count::default());
    }

    #[test]
    fn test_advance_ball() {
        let pa = PlateAppearance::new();
        let pa = pa.advance(PitchOutcome::Ball);

        if let PlateAppearanceResult::InProgress(pa) = pa {
            assert_eq!(pa.count().balls(), Balls::One);
            assert_eq!(pa.count().strikes(), Strikes::Zero);
        } else {
            panic!("Expected in-progress plate appearance");
        }
    }

    #[test]
    fn test_advance_strike() {
        let pa = PlateAppearance::new();
        let pa = pa.advance(PitchOutcome::Strike);

        if let PlateAppearanceResult::InProgress(pa) = pa {
            assert_eq!(pa.count().balls(), Balls::Zero);
            assert_eq!(pa.count().strikes(), Strikes::One);
        } else {
            panic!("Expected in-progress plate appearance");
        }
    }

    #[test]
    fn test_advance_foul() {
        let pa = PlateAppearance::new();
        let pa = pa.advance(PitchOutcome::Foul);

        if let PlateAppearanceResult::InProgress(pa) = pa {
            assert_eq!(pa.count().balls(), Balls::Zero);
            assert_eq!(pa.count().strikes(), Strikes::One);
        } else {
            panic!("Expected in-progress plate appearance");
        }
    }

    #[test]
    fn test_advance_foul_with_two_strikes() {
        let count = Count::new(Balls::Two, Strikes::Two);
        let pa = PlateAppearance::with_count(count);
        let pa = pa.advance(PitchOutcome::Foul);

        if let PlateAppearanceResult::InProgress(pa) = pa {
            assert_eq!(pa.count().balls(), Balls::Two);
            assert_eq!(pa.count().strikes(), Strikes::Two);
        } else {
            panic!("Expected in-progress plate appearance");
        }
    }

    #[test]
    fn test_walk() {
        let count = Count::new(Balls::Three, Strikes::One);
        let pa = PlateAppearance::with_count(count);
        let pa = pa.advance(PitchOutcome::Ball);

        assert!(pa.is_complete());
        assert!(matches!(pa, PlateAppearanceResult::Walk));
    }

    #[test]
    fn test_strikeout() {
        let count = Count::new(Balls::One, Strikes::Two);
        let pa = PlateAppearance::with_count(count);
        let pa = pa.advance(PitchOutcome::Strike);

        assert!(pa.is_complete());
        assert!(matches!(pa, PlateAppearanceResult::Strikeout));
    }

    #[test]
    fn test_hit_by_pitch() {
        let pa = PlateAppearance::new();
        let pa = pa.advance(PitchOutcome::HitByPitch);

        assert!(pa.is_complete());
        assert!(matches!(pa, PlateAppearanceResult::HitByPitch));
    }

    #[test]
    fn test_single() {
        let pa = PlateAppearance::new();
        let pa = pa.advance(PitchOutcome::InPlay(PlayOutcome::single(
            BaserunnerState::empty(),
            BattingPosition::First,
        )));

        assert!(pa.is_complete());
        if let PlateAppearanceResult::InPlay(outcome) = pa {
            assert_eq!(outcome.first(), BaseOutcome::Runner(BattingPosition::First))
        } else {
            panic!("Expected single");
        }
    }

    #[test]
    fn test_home_run() {
        let count = Count::new(Balls::Two, Strikes::Two);
        let pa = PlateAppearance::with_count(count);
        let pa = pa.advance(PitchOutcome::HomeRun);

        assert!(pa.is_complete());
        assert!(matches!(pa, PlateAppearanceResult::HomeRun));
    }

    #[test]
    fn test_full_count_scenarios() {
        // Test a full count walk
        let pa = PlateAppearance::new();
        let pa_advance = pa.advance(PitchOutcome::Ball); // 1-0

        let pa_advance = if let PlateAppearanceResult::InProgress(pa) = pa_advance {
            pa.advance(PitchOutcome::Strike) // 1-1
        } else {
            panic!("Expected in-progress");
        };

        let pa_advance = if let PlateAppearanceResult::InProgress(pa) = pa_advance {
            pa.advance(PitchOutcome::Ball) // 2-1
        } else {
            panic!("Expected in-progress");
        };

        let pa_advance = if let PlateAppearanceResult::InProgress(pa) = pa_advance {
            pa.advance(PitchOutcome::Strike) // 2-2
        } else {
            panic!("Expected in-progress");
        };

        let pa_advance = if let PlateAppearanceResult::InProgress(pa) = pa_advance {
            pa.advance(PitchOutcome::Ball) // 3-2
        } else {
            panic!("Expected in-progress");
        };

        let final_advance = if let PlateAppearanceResult::InProgress(pa) = pa_advance {
            pa.advance(PitchOutcome::Ball) // Walk
        } else {
            panic!("Expected in-progress");
        };

        assert!(final_advance.is_complete());
        assert!(matches!(final_advance, PlateAppearanceResult::Walk));
    }

    #[test]
    fn demo_plate_appearance() {
        // info!("Simulating a full count walk...");

        let pa = PlateAppearance::new();
        let pitches = [
            ("Ball", PitchOutcome::Ball, Count::new(Balls::One, Strikes::Zero)),
            ("Strike", PitchOutcome::Strike, Count::new(Balls::One, Strikes::One)),
            ("Ball", PitchOutcome::Ball, Count::new(Balls::Two, Strikes::One)),
            ("Strike", PitchOutcome::Strike, Count::new(Balls::Two, Strikes::Two)),
            ("Ball", PitchOutcome::Ball, Count::new(Balls::Three, Strikes::Two)),
            ("Foul ball", PitchOutcome::Foul, Count::new(Balls::Three, Strikes::Two)),
            ("Foul ball", PitchOutcome::Foul, Count::new(Balls::Three, Strikes::Two)),
            ("Ball", PitchOutcome::Ball, Count::default()),
        ];

        let mut advance = PlateAppearanceResult::InProgress(pa);

        for (_, pitch, count) in pitches.into_iter() {
            // info!("  Pitch {}: {}", i + 1, desc);

            advance = advance.advance(pitch);

            if let Some(pa) = advance.plate_appearance() {
                assert_eq!(pa.count(), count, "Count mismatch");
            } else {
                assert_eq!(advance, PlateAppearanceResult::Walk, "expected walk");
                break;
            }
        }
    }
}
