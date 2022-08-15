use std::{path::PathBuf, process::Command, str::FromStr};

use directories::ProjectDirs;
use version::{read_pe_version, VersionInfo};

const VERSION_FILENAME: &str = "CURRENT-VERSION";
pub const EXECUTABLE_FILENAME: &str = "yizhan-node.exe";

mod version;

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
    if VersionInfo::from_str(version.as_str()).is_ok() && version_dir.exists() {
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

pub fn release_program(payload: &[u8]) -> anyhow::Result<()> {
    let program_dir = get_program_dir()?;

    let mut tmp_exe_path = program_dir.clone();
    tmp_exe_path.push("tmp");
    if !tmp_exe_path.exists() {
        std::fs::create_dir_all(&tmp_exe_path)?;
    }
    tmp_exe_path.push(EXECUTABLE_FILENAME);
    std::fs::write(&tmp_exe_path, payload)?;

    let version = read_pe_version(
        tmp_exe_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path"))?,
    )?;
    std::fs::remove_file(&tmp_exe_path)?;

    let mut exe_path = program_dir.clone();
    exe_path.push(format!("[{}]", version.to_string()));
    if !exe_path.exists() {
        std::fs::create_dir_all(&exe_path)?;
    }
    exe_path.push(EXECUTABLE_FILENAME);
    std::fs::write(&exe_path, payload)?;

    Ok(())
}

pub fn release_bootstrap() -> anyhow::Result<()> {
    let mut program_path = get_program_dir()?;
    program_path.push(EXECUTABLE_FILENAME);

    std::fs::copy(&std::env::current_exe()?, &program_path)?;
    Ok(())
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
