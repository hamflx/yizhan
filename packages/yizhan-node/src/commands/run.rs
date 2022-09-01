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

pub(crate) async fn send_response<T: Connection>(
    node_id: Option<String>,
    conn: &T,
    self_node_id: &str,
    cmd_id: String,
    response: UserCommandResponse,
) {
    let mut node_id_list = node_id.map(|id| vec![id]).unwrap_or_default();
    if node_id_list.is_empty() {
        let peers = match conn.get_peers().await {
            Ok(v) => v,
            Err(err) => {
                warn!("An error occurred when get peers: {:?}", err);
                Vec::new()
            }
        };
        node_id_list.extend(peers);
    }
    for node_id in node_id_list {
        if node_id != *self_node_id {
            info!("Sending response to peer {:?}", node_id);
            match conn
                .send(
                    node_id.clone(),
                    &Message::CommandResponse(
                        Some(node_id.clone()),
                        cmd_id.clone(),
                        response.clone(),
                    ),
                )
                .await
            {
                Ok(_) => info!("Response sent"),
                Err(err) => warn!(
                    "An error occurred when sending response to {}, {:?}",
                    node_id, err
                ),
            }
        }
    }
}
