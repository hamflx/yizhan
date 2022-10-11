use std::{
    collections::HashMap,
    ffi::CStr,
    fmt::Display,
    fs::ReadDir,
    io::{self, Cursor, Write},
    mem::size_of,
    ops::Deref,
    path::PathBuf,
    ptr::null_mut,
    str::FromStr,
    sync::Arc,
};

use bincode::{Decode, Encode};
use openssl::{
    hash::MessageDigest,
    pkcs5::pbkdf2_hmac,
    pkey::PKey,
    sign::Signer,
    symm::{Cipher, Crypter, Mode},
};
use serde::{Deserialize, Serialize};
use sysinfo::{PidExt, ProcessExt, System, SystemExt};
use widestring::{WideCStr, WideCString};
use windows_sys::Win32::{
    Foundation::{CloseHandle, GetLastError, HINSTANCE, S_OK},
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
    UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath, CSIDL_PERSONAL},
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub(crate) struct WeChatPrivateInfo {
    pub(crate) key: Vec<u8>,
    pub(crate) name: String,
    pub(crate) account: String,
    pub(crate) wxid: String,
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
            .unwrap_or_else(|| panic!("未找到该版本 [{}] 的地址信息", file_version));

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
        ))
        .unwrap_or_default();
        let account = read_null_ter_string(ProcessMemoryAddress(
            handle.clone(),
            base_address + address_list[1],
        ))
        .unwrap_or_default();
        let mobile_phone = read_null_ter_string(ProcessMemoryAddress(
            handle.clone(),
            base_address + address_list[2],
        ))
        .unwrap_or_default();
        let email = read_null_ter_string(ProcessMemoryAddress(
            handle.clone(),
            base_address + address_list[3],
        ))
        .unwrap_or_default();
        let key = ProcessMemory(
            ProcessMemoryAddress(handle.clone(), base_address + address_list[4]).read_ptr_32()?,
            0x20,
        )
        .read()
        .unwrap_or_default();
        let wxid = address_list
            .get(5)
            .and_then(|addr| {
                ProcessMemoryAddress(handle, base_address + addr)
                    .read_ptr_32()
                    .ok()
            })
            .and_then(|addr| read_null_ter_string(addr).ok())
            .unwrap_or_default();

        Ok(Self {
            key,
            name: wechat_name,
            account,
            mobile_phone,
            email,
            wxid,
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
    let size = 50;
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

    pub(crate) fn read_ptr_32(&self) -> anyhow::Result<ProcessMemoryAddress> {
        self.read(size_of::<u32>()).map(|bytes| {
            ProcessMemoryAddress(self.0.clone(), unsafe { *(bytes.as_ptr() as *const u32) }
                as _)
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
            vec![
                0x2535848, 0x2535B70, 0x25357B8, 0x2532690, 0x2535B4C, 0x2535B88,
            ],
        ),
    ])
}

pub(crate) fn decrypt_wechat_db_file(key: &[u8], db_content: &[u8]) -> anyhow::Result<Vec<u8>> {
    const KEY_SIZE: usize = 32;
    const DEFAULT_ITER: usize = 64000;
    const DEFAULT_PAGESIZE: usize = 4096;
    let mut main_key = [0; KEY_SIZE];
    let salt = &db_content[..16];
    let hash_alg = MessageDigest::sha1();
    pbkdf2_hmac(key, salt, DEFAULT_ITER, hash_alg, &mut main_key)?;

    let mac_salt = salt.iter().map(|c| c ^ 58).collect::<Vec<_>>();
    let mut mac_key = [0; KEY_SIZE];
    pbkdf2_hmac(&main_key, &mac_salt, 2, hash_alg, &mut mac_key)?;

    let first = &db_content[16..DEFAULT_PAGESIZE];

    let hmac_key = PKey::hmac(mac_key.as_slice())?;
    let mut signer = Signer::new(hash_alg, &hmac_key)?;
    signer.update(&first[..first.len() - 32])?;
    signer.update(&[1, 0, 0, 0])?;
    let sign = signer.sign_to_vec()?;

    if sign != first[first.len() - 32..first.len() - 12] {
        return Err(anyhow::anyhow!("Not match"));
    }

    let mut out_file_buffer = Vec::new();
    let mut out_file = Cursor::new(&mut out_file_buffer);
    out_file.write_all("SQLite format 3\0".as_bytes())?;

    let mut page_list = db_content.chunks(DEFAULT_PAGESIZE).collect::<Vec<_>>();
    page_list[0] = &page_list[0][16..];
    let cipher = Cipher::aes_256_cbc();
    let mut buffer = vec![0; DEFAULT_PAGESIZE + cipher.block_size()];
    for page in page_list {
        let data = &page[..page.len() - 48];
        let iv = &page[page.len() - 48..page.len() - 32];
        let mut decrypter = Crypter::new(cipher, Mode::Decrypt, &main_key, Some(iv))?;

        decrypter.pad(false);
        let count = decrypter.update(data, buffer.as_mut_slice())?;
        let rest = decrypter.finalize(&mut buffer[count..])?;

        out_file.write_all(&buffer[..count + rest])?;
        out_file.write_all(&page[page.len() - 48..])?;
    }

    out_file.flush()?;

    Ok(out_file_buffer)
}

pub(crate) fn get_documents_dir() -> anyhow::Result<String> {
    let mut path_ptr = null_mut();
    match unsafe {
        SHGetKnownFolderPath(&FOLDERID_Documents, CSIDL_PERSONAL as _, 0, &mut path_ptr)
    } {
        S_OK => Ok(unsafe { WideCStr::from_ptr_str(path_ptr) }.to_string()?),
        err => Err(anyhow::anyhow!("SHGetKnownFolderPath error: {:?}", err)),
    }
}

#[derive(Debug)]
pub(crate) enum WxDbFileType {
    MSG(usize),
    MediaMSG(usize),
    MicroMsg,
    Misc,
}

impl FromStr for WxDbFileType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const MSG: &str = "MSG";
        const MEDIA_MSG: &str = "MediaMSG";
        if s.starts_with(MSG) {
            Ok(WxDbFileType::MSG(
                s[MSG.len()..].parse().map_err(|_| "Invalid Index")?,
            ))
        } else if s.starts_with(MEDIA_MSG) {
            Ok(WxDbFileType::MediaMSG(
                s[MEDIA_MSG.len()..].parse().map_err(|_| "Invalid Index")?,
            ))
        } else if s == "MicroMsg" {
            Ok(WxDbFileType::MicroMsg)
        } else if s == "Misc" {
            Ok(WxDbFileType::Misc)
        } else {
            Err("Invalid file name")
        }
    }
}

impl ToString for WxDbFileType {
    fn to_string(&self) -> String {
        match *self {
            WxDbFileType::MSG(n) => format!("MSG{}", n),
            WxDbFileType::MediaMSG(n) => format!("MediaMSG{}", n),
            WxDbFileType::MicroMsg => format!("MicroMsg"),
            WxDbFileType::Misc => format!("Misc"),
        }
    }
}

pub(crate) struct WxDbFiles(ReadDir, ReadDir);

#[derive(Debug)]
pub(crate) struct WxDbFile {
    pub(crate) file_name: String,
    pub(crate) path: PathBuf,
    pub(crate) _db_type: WxDbFileType,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Encode, Decode)]
pub(crate) struct DecryptedDbFile {
    pub(crate) file_name: String,
    pub(crate) index: usize,
    pub(crate) bytes: Vec<u8>,
}

impl WxDbFiles {
    pub(crate) fn new(wxid: &str) -> anyhow::Result<Self> {
        let docs_dir = get_documents_dir()?;
        let msg_db_dir = format!(r"{}\WeChat Files\{}\Msg\Multi", docs_dir, wxid);
        let info_db_dir = format!(r"{}\WeChat Files\{}\Msg", docs_dir, wxid);
        let msg_dir = std::fs::read_dir(msg_db_dir)?;
        let info_dir = std::fs::read_dir(info_db_dir)?;
        Ok(Self(msg_dir, info_dir))
    }
}

impl Iterator for WxDbFiles {
    type Item = anyhow::Result<WxDbFile>;

    fn next(&mut self) -> Option<Self::Item> {
        let dir = (&mut self.0).chain(&mut self.1);
        for entry in dir {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => return Some(Err(err.into())),
            };
            let file_name = entry.file_name();
            let file_name = file_name.to_str();
            if let Some(file_name) = file_name {
                let is_db = file_name.ends_with(".db");
                if is_db {
                    let file_name_without_ext = &file_name[..file_name.len() - 3];
                    if let Ok(db_type) = WxDbFileType::from_str(file_name_without_ext) {
                        let db_file = WxDbFile {
                            _db_type: db_type,
                            path: entry.path(),
                            file_name: file_name.to_string(),
                        };
                        return Some(Ok(db_file));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use crate::dump::{decrypt_wechat_db_file, WeChatPrivateInfo, WxDbFiles};

    use super::{auto_find_wechat_info, get_documents_dir};

    #[test]
    fn test_find_db_file() {
        assert!(get_documents_dir().is_ok());
    }

    #[test]
    fn test_find() {
        assert!(auto_find_wechat_info().is_ok());
    }

    #[test]
    fn test_decrypt() {
        let WeChatPrivateInfo { key, wxid, .. } = auto_find_wechat_info().unwrap();
        let dir = WxDbFiles::new(&wxid).unwrap();
        for db_file in dir {
            let db_file = db_file.unwrap();
            let mut out_file_path = db_file.path.parent().unwrap().to_path_buf();
            out_file_path.push(format!("{}-decrypted.db", db_file._db_type.to_string(),));

            let content = std::fs::read(db_file.path).unwrap();

            let res = decrypt_wechat_db_file(&key, content.as_slice());
            assert!(res.is_ok());

            let res = res.unwrap();
            let mut out_file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(out_file_path)
                .unwrap();
            assert!(out_file.write_all(res.as_slice()).is_ok());
        }
    }
}
