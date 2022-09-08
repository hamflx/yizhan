use std::{env, fs, num::ParseIntError, path::PathBuf, str::FromStr};

use chrono::{FixedOffset, Utc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set("InternalName", "yizhan-node.exe").set_version_info(
            winres::VersionInfo::PRODUCTVERSION,
            convert_version_str(env!("CARGO_PKG_VERSION"))?,
        );
        res.compile()?;
    }

    let offset = FixedOffset::east(8 * 3600);
    let now = (Utc::now() + offset).format("%Y%m%d%H%M%S");
    println!("cargo:rustc-env=VERSION_BUILD_NO={}", now);

    // include_str! 在文件不存在的时候会报错，所以如果文件不存在，就创建一个空的文件放在那里，确保编译通过。
    let config_path = env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut config_path = PathBuf::from_str(config_path.as_str())
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    config_path.push("yizhan.toml");
    if !config_path.exists() {
        fs::write(config_path, "").unwrap();
    }

    Ok(())
}

fn convert_version_str(version_str: &str) -> Result<u64, ParseIntError> {
    let mut iter = version_str.split('.');

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
    Ok(major << 48 | minor << 32 | rev << 16 | build)
}
