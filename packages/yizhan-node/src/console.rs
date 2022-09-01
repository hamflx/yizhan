use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

use crate::{commands::RequestCommand, context::YiZhanContext, error::YiZhanResult};

#[async_trait]
pub(crate) trait Console: Send + Sync {
    async fn run(
        &self,
        ctx: Arc<YiZhanContext>,
        sender: Sender<RequestCommand>,
    ) -> YiZhanResult<()>;
}
