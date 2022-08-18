use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::{command::Command, error::YiZhanResult};

#[async_trait]
pub(crate) trait Console {
    async fn run(&self, sender: Sender<Command>) -> YiZhanResult<()>;
}
