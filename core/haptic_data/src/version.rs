// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::fmt;

use serde::Deserialize;
use serde::Serialize;
use typeshare::typeshare;

/// The version used by `HapticData`
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Deserialize, Serialize)]
#[typeshare]
#[allow(missing_docs)]
pub struct Version {
    pub major: u32,
    #[serde(default)]
    pub minor: u32,
    #[serde(default)]
    pub patch: u32,
}

impl Version {
    /// Looks for a Version structure in the input JSON
    pub fn from_json(data: &str) -> Result<Version, serde_json::Error> {
        /// Helper that parses a top-level Version struct while ignoring the rest of the input
        #[derive(Deserialize)]
        pub struct VersionCheck {
            pub version: Version,
        }

        serde_json::from_str::<VersionCheck>(data).map(|version_check| version_check.version)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for Version {
    fn default() -> Self {
        Self {
            major: 1,
            minor: 0,
            patch: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::eq_op)]
    fn version_cmp() {
        assert!(
            Version {
                major: 2,
                minor: 0,
                patch: 0,
            } > Version {
                major: 1,
                minor: 0,
                patch: 0,
            }
        );
        assert!(
            Version {
                major: 2,
                minor: 1,
                patch: 0,
            } > Version {
                major: 2,
                minor: 0,
                patch: 0,
            }
        );
        assert!(
            Version {
                major: 2,
                minor: 2,
                patch: 0,
            } == Version {
                major: 2,
                minor: 2,
                patch: 0,
            }
        );
        assert!(
            Version {
                major: 1,
                minor: 2,
                patch: 1,
            } < Version {
                major: 1,
                minor: 2,
                patch: 2,
            }
        );
    }
}
