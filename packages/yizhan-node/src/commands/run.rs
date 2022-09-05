use std::{process::Command, thread::spawn, time::Duration};

use tokio::{sync::oneshot, time::timeout};
use tracing::{info, warn};
use yizhan_protocol::command::{UserCommandResponse, UserCommandResult};

use crate::{connection::Connection, context::YiZhanContext};

use super::common::send_response;

pub(crate) async fn do_run_command<T: Connection>(
    ctx: &YiZhanContext,
    node_id: Option<String>,
    cmd_id: String,
    conn: &T,
    program: String,
    args: Vec<String>,
) {
    info!("Running command: `{}` with {:?}", program, args);

    let (tx, rx) = oneshot::channel();
    spawn(move || {
        let mut child = Command::new(program.as_str());
        let response = match child.args(args).output() {
            Ok(output) => match std::str::from_utf8(output.stdout.as_slice()) {
                Ok(v) => UserCommandResult::Ok(UserCommandResponse::Run(v.to_string())),
                Err(err) => UserCommandResult::Err(format!("Err: {:?}", err)),
            },
            Err(err) => {
                warn!("Failed to read stdout: {:?}", err);
                UserCommandResult::Err(format!("Err: {:?}", err))
            }
        };
        let _ = tx.send(response);
    });

    let response = match timeout(Duration::from_secs(3), rx).await {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => UserCommandResult::Err(format!("{:?}", err)),
        Err(_) => UserCommandResult::Err("wait output timed out".to_string()),
    };

    send_response(node_id, conn, ctx, cmd_id, response).await;
}
