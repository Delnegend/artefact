use core::{fmt::Display, ops::Div};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleFactor {
    #[default]
    One,
    Two,
}

impl SampleFactor {
    pub fn u8(&self) -> u8 {
        match self {
            SampleFactor::One => 1,
            SampleFactor::Two => 2,
        }
    }

    pub fn u16(&self) -> u16 {
        match self {
            SampleFactor::One => 1,
            SampleFactor::Two => 2,
        }
    }

    pub fn u32(&self) -> u32 {
        match self {
            SampleFactor::One => 1,
            SampleFactor::Two => 2,
        }
    }

    pub fn usize(&self) -> usize {
        match self {
            SampleFactor::One => 1,
            SampleFactor::Two => 2,
        }
    }
}

impl Display for SampleFactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleFactor::One => write!(f, "1"),
            SampleFactor::Two => write!(f, "2"),
        }
    }
}

impl Div<SampleFactor> for SampleFactor {
    type Output = SampleFactor;

    fn div(self, rhs: SampleFactor) -> Self::Output {
        match (self, rhs) {
            (SampleFactor::Two, SampleFactor::One) => SampleFactor::Two,
            _ => SampleFactor::One,
        }
    }
}

impl PartialOrd for SampleFactor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SampleFactor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (SampleFactor::One, SampleFactor::One) => std::cmp::Ordering::Equal,
            (SampleFactor::One, SampleFactor::Two) => std::cmp::Ordering::Less,
            (SampleFactor::Two, SampleFactor::One) => std::cmp::Ordering::Greater,
            (SampleFactor::Two, SampleFactor::Two) => std::cmp::Ordering::Equal,
        }
    }
}
