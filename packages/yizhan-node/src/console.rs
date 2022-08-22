use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use yizhan_protocol::command::Command;

use crate::error::YiZhanResult;

#[async_trait]
pub(crate) trait Console: Send + Sync {
    async fn run(&self, sender: Sender<Command>) -> YiZhanResult<()>;
}
