use std::{io, ptr::null};

use tracing::{info, warn};
use widestring::WideCStr;
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE},
    System::{
        Threading::{
            CreateMutexA, OpenProcess, QueryFullProcessImageNameW, WaitForSingleObject,
            PROCESS_ALL_ACCESS,
        },
        WindowsProgramming::INFINITE,
    },
};

use crate::error::YiZhanResult;

pub(crate) struct WinMutex(HANDLE, bool);

impl WinMutex {
    pub(crate) fn new_named(name: &[u8]) -> YiZhanResult<Self> {
        let handle = unsafe { CreateMutexA(null(), 1, name.as_ptr()) };
        if handle == 0 {
            Err(anyhow::anyhow!("Error: {:?}", io::Error::last_os_error()))
        } else {
            Ok(Self(
                handle,
                unsafe { GetLastError() } == ERROR_ALREADY_EXISTS,
            ))
        }
    }

    pub(crate) fn exists(&self) -> bool {
        self.1
    }
}

impl Drop for WinMutex {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

pub(crate) fn wait_for_process(pid: u32) {
    unsafe {
        let mut buffer = vec![0; 4096];
        let mut size = buffer.len();
        let handle = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
        match handle {
            0 => warn!("OpenProcess error: {:?}", io::Error::last_os_error()),
            handle => {
                if QueryFullProcessImageNameW(
                    handle,
                    0,
                    buffer.as_mut_ptr(),
                    &mut size as *mut _ as _,
                ) != 0
                {
                    match WideCStr::from_slice_truncate(buffer.as_slice()) {
                        Ok(path) => match path.to_string() {
                            Ok(path) => {
                                if path.contains("yizhan-node") {
                                    info!("Waiting for yizhan-node({})", path);
                                    WaitForSingleObject(handle, INFINITE);
                                } else {
                                    warn!("not contains yizhan-node");
                                }
                            }
                            Err(err) => warn!("path.to_string error: {:?}", err),
                        },
                        Err(err) => warn!("WideCStr::from_slice_truncate error: {:?}", err),
                    }
                } else {
                    warn!(
                        "QueryFullProcessImageNameW error: {:?}",
                        io::Error::last_os_error()
                    );
                }
            }
        }
    }
}
