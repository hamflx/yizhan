use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use yizhan_protocol::message::Message;

use crate::error::YiZhanResult;

#[async_trait]
pub(crate) trait Serve {
    async fn run(&self, name: &str, sender: Sender<Message>) -> YiZhanResult<Message>;

    async fn get_peers(&self) -> YiZhanResult<Vec<String>>;

    async fn send(&self, client_id: String, message: &Message) -> YiZhanResult<()>;
}
