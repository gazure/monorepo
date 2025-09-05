use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BattingPosition {
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
}

impl Display for BattingPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BattingPosition::First => write!(f, "First"),
            BattingPosition::Second => write!(f, "Second"),
            BattingPosition::Third => write!(f, "Third"),
            BattingPosition::Fourth => write!(f, "Fourth"),
            BattingPosition::Fifth => write!(f, "Fifth"),
            BattingPosition::Sixth => write!(f, "Sixth"),
            BattingPosition::Seventh => write!(f, "Seventh"),
            BattingPosition::Eighth => write!(f, "Eighth"),
            BattingPosition::Ninth => write!(f, "Ninth"),
        }
    }
}

impl BattingPosition {
    pub fn next(self) -> BattingPosition {
        match self {
            BattingPosition::First => BattingPosition::Second,
            BattingPosition::Second => BattingPosition::Third,
            BattingPosition::Third => BattingPosition::Fourth,
            BattingPosition::Fourth => BattingPosition::Fifth,
            BattingPosition::Fifth => BattingPosition::Sixth,
            BattingPosition::Sixth => BattingPosition::Seventh,
            BattingPosition::Seventh => BattingPosition::Eighth,
            BattingPosition::Eighth => BattingPosition::Ninth,
            BattingPosition::Ninth => BattingPosition::First,
        }
    }

    pub fn num(self) -> u8 {
        match self {
            BattingPosition::First => 1,
            BattingPosition::Second => 2,
            BattingPosition::Third => 3,
            BattingPosition::Fourth => 4,
            BattingPosition::Fifth => 5,
            BattingPosition::Sixth => 6,
            BattingPosition::Seventh => 7,
            BattingPosition::Eighth => 8,
            BattingPosition::Ninth => 9,
        }
    }
}

impl From<BattingPosition> for u8 {
    fn from(value: BattingPosition) -> Self {
        value.num()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PlayerPosition {
    Pitcher,
    Catcher,
    FirstBase,
    SecondBase,
    ThirdBase,
    Shortstop,
    LeftField,
    CenterField,
    RightField,
    DesignatedHitter,
}

impl PlayerPosition {
    pub fn abbreviation(self) -> String {
        match self {
            PlayerPosition::Pitcher => "P".to_string(),
            PlayerPosition::Catcher => "C".to_string(),
            PlayerPosition::FirstBase => "1B".to_string(),
            PlayerPosition::SecondBase => "2B".to_string(),
            PlayerPosition::ThirdBase => "3B".to_string(),
            PlayerPosition::Shortstop => "SS".to_string(),
            PlayerPosition::LeftField => "LF".to_string(),
            PlayerPosition::CenterField => "CF".to_string(),
            PlayerPosition::RightField => "RF".to_string(),
            PlayerPosition::DesignatedHitter => "DH".to_string(),
        }
    }

    pub fn number(self) -> u8 {
        match self {
            PlayerPosition::Pitcher => 1,
            PlayerPosition::Catcher => 2,
            PlayerPosition::FirstBase => 3,
            PlayerPosition::SecondBase => 4,
            PlayerPosition::ThirdBase => 5,
            PlayerPosition::Shortstop => 6,
            PlayerPosition::LeftField => 7,
            PlayerPosition::CenterField => 8,
            PlayerPosition::RightField => 9,
            PlayerPosition::DesignatedHitter => 0,
        }
    }
}

impl Display for PlayerPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerPosition::Pitcher => write!(f, "Pitcher"),
            PlayerPosition::Catcher => write!(f, "Catcher"),
            PlayerPosition::FirstBase => write!(f, "First Base"),
            PlayerPosition::SecondBase => write!(f, "Second Base"),
            PlayerPosition::ThirdBase => write!(f, "Third Base"),
            PlayerPosition::Shortstop => write!(f, "Shortstop"),
            PlayerPosition::LeftField => write!(f, "Left Field"),
            PlayerPosition::CenterField => write!(f, "Center Field"),
            PlayerPosition::RightField => write!(f, "Right Field"),
            PlayerPosition::DesignatedHitter => write!(f, "Batter"),
        }
    }
}

#[cfg(test)]
mod test {
    use tracing::info;

    use super::*;

    #[test]
    fn test_next() {
        assert_eq!(BattingPosition::First.next(), BattingPosition::Second);
        assert_eq!(BattingPosition::Second.next(), BattingPosition::Third);
        assert_eq!(BattingPosition::Third.next(), BattingPosition::Fourth);
        assert_eq!(BattingPosition::Fourth.next(), BattingPosition::Fifth);
        assert_eq!(BattingPosition::Fifth.next(), BattingPosition::Sixth);
        assert_eq!(BattingPosition::Sixth.next(), BattingPosition::Seventh);
        assert_eq!(BattingPosition::Seventh.next(), BattingPosition::Eighth);
        assert_eq!(BattingPosition::Eighth.next(), BattingPosition::Ninth);
        assert_eq!(BattingPosition::Ninth.next(), BattingPosition::First);
    }

    #[test]
    fn demo_batting_position_api() {
        info!("Creating batting positions - no Result unwrapping needed!");

        // Clean enum-based creation
        let leadoff = BattingPosition::First;
        let cleanup = BattingPosition::Fourth;
        let nine_hole = BattingPosition::Ninth;

        info!("  Leadoff hitter: #{}", leadoff.num());
        info!("  Cleanup hitter: #{}", cleanup.num());
        info!("  Nine hole: #{}", nine_hole.num());

        info!("Batting order progression:");
        let mut current = BattingPosition::Seventh;
        for i in 1..=5 {
            info!("  Batter {}: #{}", i, current.num());
            current = current.next();
        }

        info!("No more .unwrap() calls needed! 🎉");
    }
}
