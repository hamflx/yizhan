use tracing::{info, warn};
use yizhan_protocol::{command::UserCommandResponse, message::Message};

use crate::connection::Connection;

pub(crate) async fn send_response<T: Connection>(
    node_id: Option<String>,
    conn: &T,
    self_node_id: &str,
    cmd_id: String,
    response: UserCommandResponse,
) {
    let node_id_list = resolve_targets(node_id, conn).await;
    send_msg_to(node_id_list, conn, self_node_id, move |node_id| {
        Message::CommandResponse(Some(node_id), cmd_id.clone(), response.clone())
    })
    .await;
}

pub(crate) async fn send_msg_to<T: Connection, F: Fn(String) -> Message>(
    node_id_list: Vec<String>,
    conn: &T,
    self_node_id: &str,
    msg: F,
) {
    for node_id in node_id_list {
        if node_id != *self_node_id {
            info!("Sending response to peer {:?}", node_id);
            match conn.send(node_id.clone(), &msg(node_id.clone())).await {
                Ok(_) => info!("Response sent"),
                Err(err) => warn!(
                    "An error occurred when sending response to {}, {:?}",
                    node_id, err
                ),
            }
        }
    }
}

pub(crate) async fn resolve_targets<T: Connection>(
    node_id: Option<String>,
    conn: &T,
) -> Vec<String> {
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
    node_id_list
}
