use core::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub enum SampleFactor {
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
