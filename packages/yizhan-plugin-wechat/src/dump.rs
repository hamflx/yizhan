use std::{
    collections::HashMap, ffi::CStr, fmt::Display, io, mem::size_of, ops::Deref, ptr::null_mut,
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use sysinfo::{PidExt, ProcessExt, System, SystemExt};
use widestring::{WideCStr, WideCString};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, HINSTANCE},
    Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, VS_FIXEDFILEINFO,
    },
    System::{
        Diagnostics::Debug::ReadProcessMemory,
        ProcessStatus::{
            K32EnumProcessModulesEx, K32GetModuleFileNameExW, K32GetModuleInformation,
            LIST_MODULES_32BIT, MODULEINFO,
        },
        Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_ALL_ACCESS},
    },
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub(crate) struct WeChatPrivateInfo {
    pub(crate) key: Vec<u8>,
    pub(crate) name: String,
    pub(crate) account: String,
    pub(crate) mobile_phone: String,
    pub(crate) email: String,
}

impl WeChatPrivateInfo {
    fn from_process(handle: Arc<ProcessHandle>) -> anyhow::Result<Self> {
        let address_info = get_address_info();

        let file_version = ExecutableFile(handle.image_name()?).get_version()?;
        let file_version = file_version.to_string();
        let address_list = address_info
            .get(file_version.as_str())
            .expect("未找到该版本的地址信息");

        let module_list = handle.module_list()?;
        let wechat_dll_name = "WeChatWin.dll".to_ascii_lowercase();
        let wechat_dll_module = module_list
            .into_iter()
            .find(|m| {
                m.module_name
                    .to_ascii_lowercase()
                    .contains(&wechat_dll_name)
            })
            .ok_or_else(|| anyhow::anyhow!("未找到 WeChatWin.dll 模块"))?;
        let base_address = wechat_dll_module.base_address as usize;

        let wechat_name = read_null_ter_string(ProcessMemoryAddress(
            handle.clone(),
            base_address + address_list[0],
        ))?;
        let account = read_null_ter_string(ProcessMemoryAddress(
            handle.clone(),
            base_address + address_list[1],
        ))?;
        let mobile_phone = read_null_ter_string(ProcessMemoryAddress(
            handle.clone(),
            base_address + address_list[2],
        ))?;
        let email = read_null_ter_string(ProcessMemoryAddress(
            handle.clone(),
            base_address + address_list[3],
        ))?;
        let key = ProcessMemory(
            ProcessMemoryAddress(handle, base_address + address_list[4]).read_ptr()?,
            0x20,
        )
        .read()?;

        Ok(Self {
            key,
            name: wechat_name,
            account,
            mobile_phone,
            email,
        })
    }
}

pub(crate) fn auto_find_wechat_info() -> anyhow::Result<WeChatPrivateInfo> {
    let pid = find_process_id("WeChat.exe").ok_or_else(|| anyhow::anyhow!("No process found"))?;
    WeChatPrivateInfo::from_process(Arc::new(pid.into()))
}

impl Display for WeChatPrivateInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Key={} Name={} Id={} Email={} Phone={}",
            to_hex_string(self.key.as_slice()),
            self.name,
            self.account,
            self.email,
            self.mobile_phone
        ))
    }
}

fn read_null_ter_string(address: ProcessMemoryAddress) -> anyhow::Result<String> {
    let size = 100;
    let memory = ProcessMemory(address, size);
    let bytes = memory.read()?;
    if bytes.iter().any(|b| *b == 0) {
        Ok(unsafe { CStr::from_ptr(bytes.as_ptr() as _) }
            .to_str()?
            .to_string())
    } else {
        Ok(std::str::from_utf8(bytes.as_slice())?.to_string())
    }
}

struct Pid(u32);

impl Pid {}

#[derive(Debug)]
struct ProcessHandle(isize);

impl ProcessHandle {
    fn image_name(&self) -> anyhow::Result<String> {
        let mut buffer = vec![0; 1024];
        let mut size = buffer.len() as u32;
        while unsafe { QueryFullProcessImageNameW(self.0, 0, buffer.as_mut_ptr(), &mut size) == 0 }
        {
            if unsafe { GetLastError() != 122 } {
                return Err(io::Error::last_os_error().into());
            }
            size *= 2;
            buffer.resize(size as _, 0);
        }

        Ok(unsafe { WideCStr::from_ptr_str(buffer.as_ptr()) }.to_string()?)
    }

    fn module_list(&self) -> anyhow::Result<Vec<ProcessModule>> {
        let mut needed = 0;
        if unsafe {
            K32EnumProcessModulesEx(self.0, null_mut(), 0, &mut needed, LIST_MODULES_32BIT)
        } == 0
        {
            return Err(io::Error::last_os_error().into());
        }

        let mut buffer = vec![0u8; needed as _];
        if unsafe {
            K32EnumProcessModulesEx(
                self.0,
                buffer.as_mut_ptr() as _,
                buffer.len() as _,
                &mut needed,
                LIST_MODULES_32BIT,
            )
        } == 0
        {
            return Err(io::Error::last_os_error().into());
        }

        let module_handles = unsafe {
            std::slice::from_raw_parts(
                buffer.as_ptr() as *const HINSTANCE,
                buffer.len() / size_of::<HINSTANCE>(),
            )
        };
        let mut module_list = Vec::new();
        let mut mod_info = MODULEINFO {
            EntryPoint: null_mut(),
            SizeOfImage: 0,
            lpBaseOfDll: null_mut(),
        };
        let mut filename_buffer = vec![0; 4096];
        for handle in module_handles {
            if unsafe {
                K32GetModuleInformation(
                    self.0,
                    *handle,
                    &mut mod_info,
                    size_of::<MODULEINFO>() as _,
                )
            } == 0
            {
                return Err(io::Error::last_os_error().into());
            }
            let filename_len = unsafe {
                K32GetModuleFileNameExW(
                    self.0,
                    *handle,
                    filename_buffer.as_mut_ptr(),
                    filename_buffer.len() as _,
                )
            };
            if filename_len == 0 {
                return Err(io::Error::last_os_error().into());
            }
            module_list.push(ProcessModule {
                base_address: mod_info.lpBaseOfDll as _,
                module_name: unsafe {
                    WideCStr::from_ptr(filename_buffer.as_ptr(), filename_len as _)
                }?
                .to_string()?,
            });
        }
        Ok(module_list)
    }
}

impl Deref for ProcessHandle {
    type Target = isize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Pid> for ProcessHandle {
    fn from(pid: Pid) -> Self {
        Self(unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid.0) })
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

#[derive(Debug)]
struct ProcessModule {
    base_address: HINSTANCE,
    module_name: String,
}

fn find_process_id(exe_name: &str) -> Option<Pid> {
    let mut system = System::new();
    system.refresh_processes();
    let lower_exe_name = exe_name.to_ascii_lowercase();
    system
        .processes()
        .values()
        .find(|process| process.name().to_ascii_lowercase() == lower_exe_name)
        .map(|p| Pid(p.pid().as_u32()))
}

#[derive(Debug)]
struct ProcessMemoryAddress(Arc<ProcessHandle>, usize);

impl ProcessMemoryAddress {
    pub(crate) fn read(&self, size: usize) -> anyhow::Result<Vec<u8>> {
        let mut buffer = vec![0; size];
        let mut bytes_read = 0;
        if unsafe {
            ReadProcessMemory(
                **self.0,
                self.1 as _,
                buffer.as_mut_ptr() as _,
                buffer.len(),
                &mut bytes_read,
            )
        } == 0
        {
            return Err(io::Error::last_os_error().into());
        }
        Ok(buffer)
    }

    pub(crate) fn read_ptr(&self) -> anyhow::Result<ProcessMemoryAddress> {
        self.read(size_of::<*const ()>()).map(|bytes| {
            ProcessMemoryAddress(
                self.0.clone(),
                unsafe { *(bytes.as_ptr() as *const *const ()) } as _,
            )
        })
    }
}

struct ProcessMemory(ProcessMemoryAddress, usize);

impl ProcessMemory {
    pub(crate) fn read(&self) -> anyhow::Result<Vec<u8>> {
        self.0.read(self.1)
    }
}

struct ExecutableFile(String);

impl ExecutableFile {
    pub(crate) fn get_version(&self) -> anyhow::Result<FileVersion> {
        let filename = WideCString::from_str(&self.0)?;
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

        Ok(FileVersion {
            file_version_ms: version_info.dwFileVersionMS,
            file_version_ls: version_info.dwFileVersionLS,
        })
    }
}

#[derive(Debug)]
struct FileVersion {
    pub(crate) file_version_ms: u32,
    pub(crate) file_version_ls: u32,
}

impl ToString for FileVersion {
    fn to_string(&self) -> String {
        format!(
            "{}.{}.{}.{}",
            (self.file_version_ms >> 16) & 0xffff,
            self.file_version_ms & 0xffff,
            (self.file_version_ls >> 16) & 0xffff,
            self.file_version_ls & 0xffff
        )
    }
}

fn to_hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|ch| format!("{:02X}", ch)).collect()
}

fn get_address_info() -> HashMap<&'static str, Vec<usize>> {
    HashMap::from([
        (
            "3.2.1.154",
            vec![328121948, 328122328, 328123056, 328121976, 328123020],
        ),
        (
            "3.3.0.115",
            vec![31323364, 31323744, 31324472, 31323392, 31324436],
        ),
        (
            "3.3.0.84",
            vec![31315212, 31315592, 31316320, 31315240, 31316284],
        ),
        (
            "3.3.0.93",
            vec![31323364, 31323744, 31324472, 31323392, 31324436],
        ),
        (
            "3.3.5.34",
            vec![30603028, 30603408, 30604120, 30603056, 30604100],
        ),
        (
            "3.3.5.42",
            vec![30603012, 30603392, 30604120, 30603040, 30604084],
        ),
        (
            "3.3.5.46",
            vec![30578372, 30578752, 30579480, 30578400, 30579444],
        ),
        (
            "3.4.0.37",
            vec![31608116, 31608496, 31609224, 31608144, 31609188],
        ),
        (
            "3.4.0.38",
            vec![31604044, 31604424, 31605152, 31604072, 31605116],
        ),
        (
            "3.4.0.50",
            vec![31688500, 31688880, 31689608, 31688528, 31689572],
        ),
        (
            "3.4.0.54",
            vec![31700852, 31701248, 31700920, 31700880, 31701924],
        ),
        (
            "3.4.5.27",
            vec![32133788, 32134168, 32134896, 32133816, 32134860],
        ),
        (
            "3.4.5.45",
            vec![32147012, 32147392, 32147064, 32147040, 32148084],
        ),
        (
            "3.5.0.20",
            vec![35494484, 35494864, 35494536, 35494512, 35495556],
        ),
        (
            "3.5.0.29",
            vec![35507980, 35508360, 35508032, 35508008, 35509052],
        ),
        (
            "3.5.0.33",
            vec![35512140, 35512520, 35512192, 35512168, 35513212],
        ),
        (
            "3.5.0.39",
            vec![35516236, 35516616, 35516288, 35516264, 35517308],
        ),
        (
            "3.5.0.42",
            vec![35512140, 35512520, 35512192, 35512168, 35513212],
        ),
        (
            "3.5.0.44",
            vec![35510836, 35511216, 35510896, 35510864, 35511908],
        ),
        (
            "3.5.0.46",
            vec![35506740, 35507120, 35506800, 35506768, 35507812],
        ),
        (
            "3.6.0.18",
            vec![35842996, 35843376, 35843048, 35843024, 35844068],
        ),
        (
            "3.6.5.7",
            vec![35864356, 35864736, 35864408, 35864384, 35865428],
        ),
        (
            "3.6.5.16",
            vec![35909428, 35909808, 35909480, 35909456, 35910500],
        ),
        (
            "3.7.0.26",
            vec![37105908, 37106288, 37105960, 37105936, 37106980],
        ),
        (
            "3.7.0.29",
            vec![37105908, 37106288, 37105960, 37105936, 37106980],
        ),
        (
            "3.7.0.30",
            vec![37118196, 37118576, 37118248, 37118224, 37119268],
        ),
        (
            "3.7.5.11",
            vec![37883280, 37884088, 37883136, 37883008, 37884052],
        ),
        (
            "3.7.5.23",
            vec![37895736, 37896544, 37895592, 37883008, 37896508],
        ),
        (
            "3.7.5.27",
            vec![37895736, 37896544, 37895592, 37895464, 37896508],
        ),
        (
            "3.7.5.31",
            vec![37903928, 37904736, 37903784, 37903656, 37904700],
        ),
        (
            "3.7.6.44",
            // todo 地址似乎不太对。
            vec![0x2535848, 0x2535B88, 0x25357B8, 0x2535BA8, 0x2535B4C],
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::auto_find_wechat_info;

    #[test]
    fn test_find() {
        println!("{:?}", auto_find_wechat_info());
        assert!(auto_find_wechat_info().is_ok());
    }
}
