use async_trait::async_trait;
use yizhan_protocol::message::Message;

use crate::error::YiZhanResult;

#[async_trait]
pub(crate) trait Serve {
    async fn run(&self) -> YiZhanResult<Message>;

    async fn send(&self, message: &Message);
}
