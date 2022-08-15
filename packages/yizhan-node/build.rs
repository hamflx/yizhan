use std::num::ParseIntError;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set("InternalName", "yizhan-node.exe").set_version_info(
            winres::VersionInfo::PRODUCTVERSION,
            convert_version_str(env!("CARGO_PKG_VERSION"))?,
        );
        res.compile()?;
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
