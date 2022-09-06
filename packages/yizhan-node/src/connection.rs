use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{broadcast::Receiver, mpsc::Sender};
use yizhan_protocol::{command::ListedNodeInfo, message::Message};

use crate::{context::YiZhanContext, error::YiZhanResult};

#[async_trait]
pub(crate) trait Connection {
    async fn run(
        &self,
        ctx: Arc<YiZhanContext>,
        sender: Sender<(String, Message)>,
        shut_rx: Receiver<()>,
    ) -> YiZhanResult<()>;

    async fn get_peers(&self) -> YiZhanResult<Vec<ListedNodeInfo>>;

    async fn send(&self, client_id: String, message: Message) -> YiZhanResult<()>;

    async fn flush(&self) -> YiZhanResult<()>;
}
