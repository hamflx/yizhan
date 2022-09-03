use std::process::Command;

use tracing::{info, warn};
use yizhan_protocol::command::{CommandRunResult, UserCommandResponse};

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
    let mut child = Command::new(program.as_str());
    let response = match child.args(args).output() {
        Ok(output) => match std::str::from_utf8(output.stdout.as_slice()) {
            Ok(v) => CommandRunResult::Success(v.to_string()),
            Err(err) => CommandRunResult::Failed(format!("Err: {:?}", err)),
        },
        Err(err) => {
            warn!("Failed to read stdout: {:?}", err);
            CommandRunResult::Failed(format!("Err: {:?}", err))
        }
    };

    send_response(
        node_id,
        conn,
        ctx,
        cmd_id,
        UserCommandResponse::Run(response),
    )
    .await;
}
