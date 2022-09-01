use std::process::Command;

use log::warn;
use yizhan_protocol::command::{CommandRunResult, UserCommandResponse};

use crate::connection::Connection;

use super::common::send_response;

pub(crate) async fn do_run_command<T: Connection>(
    self_node_id: &str,
    node_id: Option<String>,
    cmd_id: String,
    conn: &T,
    program: String,
) {
    let mut child = Command::new(program.as_str());
    let response = match child.output() {
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
        self_node_id,
        cmd_id,
        UserCommandResponse::Run(response),
    )
    .await;
}
