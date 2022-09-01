use std::{env::current_exe, fs::read};

use log::info;
use sha256::digest_bytes;
use yizhan_protocol::{
    command::{CommandUpdateResult, UserCommandResponse},
    version::VersionInfo,
};

use crate::{commands::common::send_response, connection::Connection, error::YiZhanResult};

pub(crate) fn get_current_binary() -> YiZhanResult<Vec<u8>> {
    let exe = current_exe()?;
    let content = read(exe)?;
    Ok(content)
}

pub(crate) async fn do_update_command<T: Connection>(
    self_node_id: &str,
    node_id: Option<String>,
    cmd_id: String,
    conn: &T,
    version: VersionInfo,
    sha256: String,
    bytes: Vec<u8>,
) {
    info!(
        "Got update request: {}, sha256: {}",
        version.to_string(),
        sha256
    );
    let bytes_sha256 = digest_bytes(bytes.as_slice());
    let response = if bytes_sha256 == sha256 {
        UserCommandResponse::Update(CommandUpdateResult::Success)
    } else {
        UserCommandResponse::Update(CommandUpdateResult::Failed(format!(
            "Invalid sha256, expected: {}, got: {}",
            sha256, bytes_sha256
        )))
    };

    send_response(node_id, conn, self_node_id, cmd_id, response).await
}
