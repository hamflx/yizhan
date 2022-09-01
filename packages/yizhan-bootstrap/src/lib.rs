use std::{fmt::Debug, path::PathBuf, process::Command, str::FromStr};

use directories::ProjectDirs;
use yizhan_protocol::version::VersionInfo;

const VERSION_FILENAME: &str = "CURRENT-VERSION";
#[cfg(windows)]
const EXECUTABLE_FILENAME: &str = "yizhan-node.exe";
#[cfg(unix)]
const EXECUTABLE_FILENAME: &str = "yizhan-node";

pub fn get_current_or_latest_version() -> Option<VersionInfo> {
    get_current_version().or_else(get_latest_version)
}

pub fn get_current_version() -> Option<VersionInfo> {
    let mut program_dir = get_program_dir().ok()?;
    let mut version_file = program_dir.clone();
    version_file.push(VERSION_FILENAME);
    let version = std::fs::read_to_string(&version_file).ok()?;
    program_dir.push(format!("[{}]", version));
    if VersionInfo::from_str(version.as_str()).is_ok() && program_dir.exists() {
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
    if let Ok(program_dir) = get_program_dir() {
        if let Ok(files) = std::fs::read_dir(&program_dir) {
            for path in files.flatten() {
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
    version_list
}

pub fn spawn_program() -> anyhow::Result<()> {
    let mut program_path = get_program_dir()?;
    let version =
        get_current_or_latest_version().ok_or_else(|| anyhow::anyhow!("No version found"))?;
    program_path.push(format!("[{}]", version.to_string()));
    program_path.push(EXECUTABLE_FILENAME);
    if !program_path.exists() {
        return Err(anyhow::anyhow!("Program does not exists"));
    }

    Command::new(&program_path).spawn()?;

    Ok(())
}

pub fn install_bootstrap() -> anyhow::Result<()> {
    let program_dir = get_program_dir()?;
    let current_exe = std::env::current_exe()?;

    let mut bootstrap_path = program_dir;
    bootstrap_path.push(EXECUTABLE_FILENAME);
    std::fs::copy(&current_exe, &bootstrap_path)?;

    Ok(())
}

pub fn install_program<V>(current_version: V) -> anyhow::Result<()>
where
    V: TryInto<VersionInfo>,
    V::Error: Debug,
{
    let program_dir = get_program_dir()?;
    let current_exe = std::env::current_exe()?;

    let version: VersionInfo = current_version
        .try_into()
        .map_err(|err| anyhow::anyhow!("Err: {:?}", err))?;
    let mut exe_path = program_dir;
    exe_path.push(format!("[{}]", version.to_string()));
    if !exe_path.exists() {
        std::fs::create_dir_all(&exe_path)?;
    }
    exe_path.push(EXECUTABLE_FILENAME);
    std::fs::copy(&current_exe, &exe_path)?;

    Ok(())
}

pub fn is_running_process_installed<V>(current_version: V) -> anyhow::Result<bool>
where
    V: TryInto<VersionInfo>,
    V::Error: Debug,
{
    let current_exe_path = std::env::current_exe()?;
    let current_exe_path = current_exe_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid PathBuf"))?;
    let version = current_version
        .try_into()
        .map_err(|err| anyhow::anyhow!("Err: {:?}", err))?;

    let mut program_path = get_program_dir()?;
    program_path.push(format!("[{}]", version.to_string()));
    program_path.push(EXECUTABLE_FILENAME);

    let program_path = program_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid PathBuf"))?;

    Ok(program_path.to_ascii_lowercase() == current_exe_path.to_ascii_lowercase())
}

pub fn get_project_dir() -> Option<ProjectDirs> {
    ProjectDirs::from("cn", "hamflx", "yizhan")
}

pub fn get_program_dir() -> anyhow::Result<PathBuf> {
    let project_dir =
        get_project_dir().ok_or_else(|| anyhow::anyhow!("Failed to get project dir"))?;
    let executable_dir = project_dir.data_local_dir();
    if !executable_dir.exists() {
        std::fs::create_dir_all(executable_dir)?;
    }
    Ok(executable_dir.to_path_buf())
}
