use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetPlatform {
    Vulkan(Version),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
}

impl Version {
    pub const V1_0: Version = Self::new(1, 0);
    pub const V1_1: Version = Self::new(1, 1);
    pub const V1_2: Version = Self::new(1, 2);
    pub const V1_3: Version = Self::new(1, 3);
    pub const V1_4: Version = Self::new(1, 4);
    pub const V1_5: Version = Self::new(1, 5);
    pub const V1_6: Version = Self::new(1, 6);

    pub const fn new(major: u8, minor: u8) -> Self {
        return Self { major, minor };
    }
}

impl TargetPlatform {
    pub const VK_1_0: TargetPlatform = Self::Vulkan(Version::new(1, 0));
    pub const VK_1_1: TargetPlatform = Self::Vulkan(Version::new(1, 1));
    pub const VK_1_2: TargetPlatform = Self::Vulkan(Version::new(1, 2));
    pub const VK_1_3: TargetPlatform = Self::Vulkan(Version::new(1, 3));
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
        Self::V1_0
    }
}

/* VERSION CONVERSIONS */
impl From<TargetPlatform> for Version {
    fn from(value: TargetPlatform) -> Self {
        match value {
            TargetPlatform::Vulkan(version) => {
                if version >= Version::V1_3 {
                    return Version::V1_6;
                } else if version >= Version::V1_2 {
                    return Version::V1_5;
                } else if version >= Version::V1_1 {
                    return Version::V1_3;
                } else {
                    return Version::V1_0;
                }
            }
            _ => todo!(),
        }
    }
}
