use crate::{
    error::Error,
    fg::extended_is::{ExtendedIs, ExtendedSet},
};
use docfg::docfg;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(C)]
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

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        String::serialize(&self.to_string(), serializer)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::from_str(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
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
        Self::V1_0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum TargetPlatform {
    Universal(Version),
    Vulkan(Version),
}

impl TargetPlatform {
    pub const SPV_1_0: TargetPlatform = Self::Universal(Version::V1_0);
    pub const SPV_1_1: TargetPlatform = Self::Universal(Version::V1_1);
    pub const SPV_1_2: TargetPlatform = Self::Universal(Version::V1_2);
    pub const SPV_1_3: TargetPlatform = Self::Universal(Version::V1_3);
    pub const SPV_1_4: TargetPlatform = Self::Universal(Version::V1_4);
    pub const SPV_1_5: TargetPlatform = Self::Universal(Version::V1_5);

    pub const VK_1_0: TargetPlatform = Self::Vulkan(Version::V1_0);
    pub const VK_1_1: TargetPlatform = Self::Vulkan(Version::V1_1);
    pub const VK_1_2: TargetPlatform = Self::Vulkan(Version::V1_2);

    pub fn extended_is(&self) -> Option<ExtendedIs> {
        let kind = match self {
            Self::Vulkan(_) => ExtendedSet::GLSL450,
            _ => return None,
        };
        return Some(ExtendedIs::new(kind));
    }

    pub fn is_vulkan(self) -> bool {
        return matches!(self, Self::Vulkan(_));
    }

    pub fn spirv_version(self) -> Version {
        return match self {
            TargetPlatform::Universal(version) => version,
            TargetPlatform::Vulkan(version) => {
                if version >= Version::V1_3 {
                    Version::V1_6
                } else if version >= Version::V1_2 {
                    Version::V1_5
                } else if version >= Version::V1_1 {
                    Version::V1_3
                } else {
                    Version::V1_0
                }
            }
        };
    }
}

#[docfg(feature = "spirv-tools")]
impl From<&TargetPlatform> for spirv_tools::TargetEnv {
    fn from(platform: &TargetPlatform) -> Self {
        match platform {
            &TargetPlatform::SPV_1_0 => Self::Universal_1_0,
            &TargetPlatform::SPV_1_1 => Self::Universal_1_1,
            &TargetPlatform::SPV_1_2 => Self::Universal_1_2,
            &TargetPlatform::SPV_1_3 => Self::Universal_1_3,
            &TargetPlatform::SPV_1_4 => Self::Universal_1_4,
            &TargetPlatform::SPV_1_5 => Self::Universal_1_5,

            &TargetPlatform::VK_1_0 => Self::Vulkan_1_0,
            &TargetPlatform::VK_1_1 => Self::Vulkan_1_1,
            &TargetPlatform::VK_1_2 => Self::Vulkan_1_2,
            _ => todo!(),
        }
    }
}
