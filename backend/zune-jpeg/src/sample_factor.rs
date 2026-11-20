use core::{fmt::Display, ops::Div};

/// A per-component sampling factor from the SOF header.
///
/// JPEG allows 1, 2 or 4 (4 only for horizontal, in practice, e.g. 4:1:1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleFactor {
    #[default]
    One,
    Two,
    Four,
}

impl SampleFactor {
    #[must_use]
    pub const fn value(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
        }
    }

    #[must_use]
    pub const fn u8(self) -> u8 {
        self.value()
    }

    #[must_use]
    pub const fn u16(self) -> u16 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
        }
    }

    #[must_use]
    pub const fn u32(self) -> u32 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
        }
    }

    #[must_use]
    pub const fn usize(self) -> usize {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
        }
    }
}

impl TryFrom<u8> for SampleFactor {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            4 => Ok(Self::Four),
            other => Err(other),
        }
    }
}

impl Display for SampleFactor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl Div<Self> for SampleFactor {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Four, Self::One) => Self::Four,
            (Self::Four, Self::Two) | (Self::Two, Self::One) => Self::Two,
            _ => Self::One,
        }
    }
}

impl PartialOrd for SampleFactor {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SampleFactor {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.value().cmp(&other.value())
    }
}
