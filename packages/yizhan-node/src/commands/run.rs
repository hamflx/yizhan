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
        let peers = match conn.get_peers().await {
            Ok(v) => v,
            Err(err) => {
                warn!("An error occurred when get peers: {:?}", err);
                Vec::new()
            }
        };
        node_id_list.extend(peers);
    }
    if node_id_list.is_empty() {
        warn!("No target to send command");
    } else {
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

        for node_id in node_id_list {
            if node_id != *self_node_id {
                info!("Sending response to peer {:?}", node_id);
                match conn
                    .send(
                        node_id.clone(),
                        &Message::CommandResponse(
                            Some(node_id.clone()),
                            cmd_id.clone(),
                            UserCommandResponse::Run(response.clone()),
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
}
