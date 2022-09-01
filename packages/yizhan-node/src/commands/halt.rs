use log::info;
use tokio::sync::broadcast::Sender;
use yizhan_protocol::{command::UserCommand, message::Message};

use crate::{
    commands::common::{resolve_targets, send_msg_to},
    connection::Connection,
};

pub(crate) async fn do_halt_command<T: Connection>(
    self_node_id: &str,
    node_id: Option<String>,
    cmd_id: String,
    conn: &T,
    shut_tx: &Sender<()>,
) {
    info!("Got halt command: node:{:?}", node_id);
    let node_id_list = resolve_targets(node_id, conn).await;
    send_msg_to(node_id_list, conn, self_node_id, |node_id| {
        Message::CommandRequest(Some(node_id), cmd_id.clone(), UserCommand::Halt)
    })
    .await;
    shut_tx.send(()).unwrap();
}
