use std::fmt::Display;

pub type Runs = u8;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HomePlateRuns {
    #[default]
    Zero,
    One,
    Two,
    Three,
    Four,
}

impl HomePlateRuns {
    pub fn new() -> Self {
        HomePlateRuns::Zero
    }
    pub fn to_runs(&self) -> Runs {
        match self {
            HomePlateRuns::Zero => 0,
            HomePlateRuns::One => 1,
            HomePlateRuns::Two => 2,
            HomePlateRuns::Three => 3,
            HomePlateRuns::Four => 4,
        }
    }
}

impl From<HomePlateRuns> for Runs {
    fn from(value: HomePlateRuns) -> Self {
        value.to_runs()
    }
}

impl Display for HomePlateRuns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_runs())
    }
}
