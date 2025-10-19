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
        use BattingPosition::*;
        match self {
            First => write!(f, "First"),
            Second => write!(f, "Second"),
            Third => write!(f, "Third"),
            Fourth => write!(f, "Fourth"),
            Fifth => write!(f, "Fifth"),
            Sixth => write!(f, "Sixth"),
            Seventh => write!(f, "Seventh"),
            Eighth => write!(f, "Eighth"),
            Ninth => write!(f, "Ninth"),
        }
    }
}

impl BattingPosition {
    pub fn next(self) -> BattingPosition {
        use BattingPosition::*;
        match self {
            First => Second,
            Second => Third,
            Third => Fourth,
            Fourth => Fifth,
            Fifth => Sixth,
            Sixth => Seventh,
            Seventh => Eighth,
            Eighth => Ninth,
            Ninth => First,
        }
    }

    pub fn num(self) -> u8 {
        use BattingPosition::*;
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
        use PlayerPosition::*;
        match self {
            Pitcher => "P".to_string(),
            Catcher => "C".to_string(),
            FirstBase => "1B".to_string(),
            SecondBase => "2B".to_string(),
            ThirdBase => "3B".to_string(),
            Shortstop => "SS".to_string(),
            LeftField => "LF".to_string(),
            CenterField => "CF".to_string(),
            RightField => "RF".to_string(),
            DesignatedHitter => "DH".to_string(),
        }
    }

    pub fn number(self) -> u8 {
        use PlayerPosition::*;
        match self {
            Pitcher => 1,
            Catcher => 2,
            FirstBase => 3,
            SecondBase => 4,
            ThirdBase => 5,
            Shortstop => 6,
            LeftField => 7,
            CenterField => 8,
            RightField => 9,
            DesignatedHitter => 0,
        }
    }
}

impl Display for PlayerPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use PlayerPosition::*;
        match self {
            Pitcher => write!(f, "Pitcher"),
            Catcher => write!(f, "Catcher"),
            FirstBase => write!(f, "First Base"),
            SecondBase => write!(f, "Second Base"),
            ThirdBase => write!(f, "Third Base"),
            Shortstop => write!(f, "Shortstop"),
            LeftField => write!(f, "Left Field"),
            CenterField => write!(f, "Center Field"),
            RightField => write!(f, "Right Field"),
            DesignatedHitter => write!(f, "Batter"),
        }
    }
}

#[cfg(test)]
mod test {
    use tracingx::info;

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
    }
}
