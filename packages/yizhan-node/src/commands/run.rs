use std::process::Command;

use log::{info, warn};
use yizhan_protocol::{
    command::{CommandRunResult, UserCommandResponse},
    message::Message,
};

use crate::connection::Connection;

pub(crate) async fn do_run_command<T: Connection>(
    self_node_id: &str,
    node_id: Option<String>,
    cmd_id: String,
    conn: &T,
    program: String,
) {
    let mut node_id_list = node_id.map(|id| vec![id]).unwrap_or_default();
    if node_id_list.is_empty() {
        node_id_list.extend(conn.get_peers().await.unwrap());
    }
    if node_id_list.is_empty() {
        warn!("No target to send command");
    } else {
        let mut child = Command::new(program.as_str());
        let response = match child.output() {
            Ok(output) => CommandRunResult::Success(
                std::str::from_utf8(output.stdout.as_slice())
                    .unwrap()
                    .to_string(),
            ),
            Err(err) => {
                warn!("Failed to read stdout: {:?}", err);
                CommandRunResult::Failed(format!("Err: {:?}", err))
            }
        };

        for node_id in node_id_list {
            if node_id != *self_node_id {
                info!("Sending response to peer {:?}", node_id);
                conn.send(
                    node_id.clone(),
                    &Message::CommandResponse(
                        Some(node_id.clone()),
                        cmd_id.clone(),
                        UserCommandResponse::Run(response.clone()),
                    ),
                )
                .await
                .unwrap();
                info!("Response sent");
            }
        }
    }
}
