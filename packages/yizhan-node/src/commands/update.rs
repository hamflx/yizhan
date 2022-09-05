use std::{env::current_exe, fs::read};

use sha256::digest_bytes;
use tokio::sync::broadcast::Sender;
use tracing::info;
use yizhan_bootstrap::{install_bootstrap, install_program, spawn_program};
use yizhan_protocol::{
    command::{UserCommandResponse, UserCommandResult},
    version::VersionInfo,
};

use crate::{
    commands::{common::send_response, current_platform},
    connection::Connection,
    context::YiZhanContext,
    error::YiZhanResult,
    network::ShutdownHooks,
};

pub(crate) fn get_current_binary() -> YiZhanResult<Vec<u8>> {
    let exe = current_exe()?;
    let content = read(exe)?;
    Ok(content)
}

pub(crate) async fn do_update_command<T: Connection>(
    ctx: &YiZhanContext,
    platform: &str,
    node_id: Option<String>,
    cmd_id: String,
    conn: &T,
    version: VersionInfo,
    sha256: String,
    bytes: Vec<u8>,
    shut_tx: &Sender<()>,
    shutdown_hooks: &ShutdownHooks,
) {
    info!(
        "Got update request: {}, sha256: {}",
        version.to_string(),
        sha256
    );
    let bytes_sha256 = digest_bytes(bytes.as_slice());
    let expected_platform = current_platform();
    let response = match (bytes_sha256 == sha256, platform == expected_platform) {
        (true, true) => UserCommandResult::Ok(UserCommandResponse::Update),
        (false, _) => UserCommandResult::Err(format!(
            "Invalid sha256, expected: {}, got: {}",
            sha256, bytes_sha256
        )),
        (_, false) => UserCommandResult::Err(format!(
            "Invalid platform, expected: {}, got: {}",
            expected_platform, platform
        )),
    };

    let should_restart = response == UserCommandResult::Ok(UserCommandResponse::Update);
    send_response(node_id, conn, ctx, cmd_id, response).await;

    if should_restart {
        shut_tx.send(()).unwrap();

        let mut shutdown_hooks = shutdown_hooks.lock().await;
        shutdown_hooks.push(Box::new(move || {
            let _ = install_bootstrap();
            let _ = install_program(&version, bytes.as_slice());
            let _ = spawn_program();
        }));
    }
}
