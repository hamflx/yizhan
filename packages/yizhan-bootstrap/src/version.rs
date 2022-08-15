use std::{io, num::ParseIntError, ptr::null_mut, str::FromStr};

use widestring::WideCString;
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
};

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

impl ToString for VersionInfo {
    fn to_string(&self) -> String {
        format!("{}.{}.{}.{}", self.0, self.1, self.2, self.3)
    }
}

pub fn read_pe_version(filename: &str) -> anyhow::Result<VersionInfo> {
    let filename = WideCString::from_str(filename)?;
    let filename_ptr = filename.as_ptr();
    let mut version_handle = 0;
    let version_size = unsafe { GetFileVersionInfoSizeW(filename_ptr, &mut version_handle) };
    if version_size == 0 {
        return Err(io::Error::last_os_error().into());
    }
    let mut version_buffer = vec![0u8; version_size as usize];
    if unsafe {
        GetFileVersionInfoW(
            filename_ptr,
            version_handle,
            version_size,
            version_buffer.as_mut_ptr() as _,
        )
    } == 0
    {
        return Err(io::Error::last_os_error().into());
    }

    let sub_block = WideCString::from_str("\\")?;
    let mut version_value_ptr: *mut VS_FIXEDFILEINFO = null_mut();
    let mut version_value_size = 0;
    if unsafe {
        VerQueryValueW(
            version_buffer.as_ptr() as _,
            sub_block.as_ptr(),
            &mut version_value_ptr as *mut *mut VS_FIXEDFILEINFO as _,
            &mut version_value_size,
        )
    } == 0
    {
        return Err(io::Error::last_os_error().into());
    }

    if version_value_size == 0 || version_value_ptr.is_null() {
        return Err(anyhow::anyhow!("No version found"));
    }

    let version_info = unsafe { &*version_value_ptr };
    if version_info.dwSignature != 0xfeef04bd {
        return Err(anyhow::anyhow!("Invalid version info"));
    }

    Ok(VersionInfo(
        ((version_info.dwFileVersionMS >> 16) & 0xffff) as _,
        (version_info.dwFileVersionMS & 0xffff) as _,
        ((version_info.dwFileVersionLS >> 16) & 0xffff) as _,
        (version_info.dwFileVersionLS & 0xffff) as _,
    ))
}
