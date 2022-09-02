use std::{num::ParseIntError, str::FromStr};

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub struct VersionInfo(u64, u64, u64, u64);

impl VersionInfo {
    pub fn set_build_no(&mut self, build: u64) {
        self.3 = build;
    }
}

impl TryFrom<&str> for VersionInfo {
    type Error = ParseIntError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        VersionInfo::from_str(value)
    }
}

impl FromStr for VersionInfo {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split('.');
        let major = if let Some(major) = iter.next() {
            str::parse(major)?
        } else {
            0
        };
        let minor = if let Some(minor) = iter.next() {
            str::parse(minor)?
        } else {
            0
        };
        let rev = if let Some(rev) = iter.next() {
            str::parse(rev)?
        } else {
            0
        };
        let build = if let Some(build) = iter.next() {
            str::parse(build)?
        } else {
            0
        };
        Ok(Self(major, minor, rev, build))
    }
}

impl PartialOrd for VersionInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.0.cmp(&other.0) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.1.cmp(&other.1) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.2.cmp(&other.2) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.3.cmp(&other.3)
    }
}

impl ToString for VersionInfo {
    fn to_string(&self) -> String {
        format!("{}.{}.{}.{}", self.0, self.1, self.2, self.3)
    }
}
