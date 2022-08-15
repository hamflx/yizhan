use std::{num::ParseIntError, str::FromStr};

const VERSION_FILENAME: &str = "CURRENT-VERSION";
const _EXECUTABLE_FILENAME: &str = "yizhan-node.exe";

#[derive(Debug, PartialEq, Eq)]
pub struct VersionInfo(usize, usize, usize, usize);

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

pub fn get_current_or_latest_version() -> Option<VersionInfo> {
    get_current_version().or_else(|| get_latest_version())
}

pub fn get_current_version() -> Option<VersionInfo> {
    let exe = std::env::current_exe().ok()?;
    let mut version_dir = exe.parent()?.to_path_buf();
    let mut version_file = exe.parent()?.to_path_buf();
    version_file.push(VERSION_FILENAME);
    let version = std::fs::read_to_string(&version_file).ok()?;
    version_dir.push(format!("[{}]", version));
    if is_valid_version(version.as_str()) && version_dir.exists() {
        VersionInfo::from_str(&version).ok()
    } else {
        None
    }
}

pub fn get_latest_version() -> Option<VersionInfo> {
    let mut version_list = get_version_list();
    if version_list.is_empty() {
        return None;
    }
    version_list.sort();
    version_list.pop()
}

pub fn get_version_list() -> Vec<VersionInfo> {
    let mut version_list = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let dir = dir.to_path_buf();
            if let Ok(files) = std::fs::read_dir(&dir) {
                for path in files {
                    if let Ok(path) = path {
                        if let Some(path) = path.file_name().to_str() {
                            if path.starts_with('[') && path.ends_with(']') {
                                let version = &path[1..path.len() - 1];
                                if let Ok(version) = version.parse() {
                                    version_list.push(version);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    version_list
}

pub fn is_valid_version(version: &str) -> bool {
    version.split('.').all(|n| str::parse::<usize>(n).is_ok())
}
