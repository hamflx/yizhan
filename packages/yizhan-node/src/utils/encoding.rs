use yizhan_common::error::YiZhanResult;

#[cfg(windows)]
pub(crate) fn native_null_ter_to_string(c_str: &[u8]) -> YiZhanResult<String> {
    use std::{ffi::OsString, io, os::windows::prelude::OsStringExt, ptr::null_mut};

    use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_ACP};

    let len = c_str.iter().position(|ch| *ch == 0).unwrap_or(c_str.len());
    let required_size =
        unsafe { MultiByteToWideChar(CP_ACP, 0, c_str.as_ptr(), len as _, null_mut(), 0) };
    if required_size <= 0 {
        return Err(io::Error::last_os_error().into());
    }
    let mut buffer = vec![0; required_size as _];
    if unsafe {
        MultiByteToWideChar(
            CP_ACP,
            0,
            c_str.as_ptr(),
            len as _,
            buffer.as_mut_ptr(),
            required_size,
        )
    } > 0
    {
        Ok(OsString::from_wide(&buffer)
            .to_str()
            .unwrap_or_default()
            .to_string())
    } else {
        Err(io::Error::last_os_error().into())
    }
}

#[cfg(not(windows))]
pub(crate) fn native_null_ter_to_string(c_str: &[u8]) -> YiZhanResult<String> {
    Ok(String::from_utf8(c_str.to_vec())?)
}
