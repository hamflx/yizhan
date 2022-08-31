use std::{env::current_exe, fs::read};

use crate::error::YiZhanResult;

pub(crate) fn get_current_binary() -> YiZhanResult<Vec<u8>> {
    let exe = current_exe()?;
    let content = read(exe)?;
    Ok(content)
}
