use yizhan_bootstrap::uninstall_program;
use yizhan_protocol::command::{UserCommandResponse, UserCommandResult};

use crate::{connection::Connection, context::YiZhanContext};

use super::common::send_response;

pub(crate) async fn do_uninstall_command<T: Connection>(
    ctx: &YiZhanContext,
    node_id: Option<String>,
    cmd_id: String,
    conn: &T,
) {
    let response = match uninstall_program() {
        Ok(_) => UserCommandResult::Ok(UserCommandResponse::Uninstall),
        Err(err) => UserCommandResult::Err(format!("Err: {:?}", err)),
    };
    send_response(node_id, conn, ctx, cmd_id, response).await;
}
