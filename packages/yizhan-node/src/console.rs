use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::{command::RequestCommand, error::YiZhanResult};

#[async_trait]
pub(crate) trait Console: Send + Sync {
    async fn run(&self, sender: Sender<RequestCommand>) -> YiZhanResult<()>;
}
