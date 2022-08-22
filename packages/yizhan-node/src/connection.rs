use async_trait::async_trait;
use yizhan_protocol::message::Message;

use crate::error::YiZhanResult;

#[async_trait]
pub(crate) trait Connection {
    async fn run(&self) -> YiZhanResult<Message>;
}
