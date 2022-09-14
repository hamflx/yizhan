use tracing::info;
use yizhan_protocol::command::{UserCommandResponse, UserCommandResult};

use crate::{commands::common::send_response, connection::Connection, context::YiZhanContext};

pub(crate) async fn do_get_command<T: Connection>(
    ctx: &YiZhanContext,
    node_id: Option<String>,
    cmd_id: String,
    conn: &T,
    path: String,
) {
    info!("Getting file `{}`", path);

    let response = match std::fs::read(path) {
        Ok(content) => UserCommandResult::Ok(UserCommandResponse::Get(content)),
        Err(err) => UserCommandResult::Err(format!("get file content error: {:?}", err)),
    };

    send_response(node_id, conn, ctx, cmd_id, response).await;
}
