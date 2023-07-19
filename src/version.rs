use std::str::FromStr;

use crate::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

impl Version {
    pub const V1: Version = Self::new(1, 0);
    pub const V1_1: Version = Self::new(1, 1);
    pub const V1_2: Version = Self::new(1, 2);
    pub const V1_3: Version = Self::new(1, 3);
    pub const V1_4: Version = Self::new(1, 4);
    pub const V1_5: Version = Self::new(1, 5);

    pub const fn new(major: u8, minor: u8) -> Self {
        return Self { major, minor };
    }
}

impl From<(u8, u8)> for Version {
    fn from((major, minor): (u8, u8)) -> Self {
        Self::new(major, minor)
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (major, minor) = s
            .split_once('.')
            .ok_or_else(|| Error::msg("Version parsing error"))?;
        Ok(Self::new(u8::from_str(major)?, u8::from_str(minor)?))
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new(1, 0)
    }
}
