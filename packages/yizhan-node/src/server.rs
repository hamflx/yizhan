use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use yizhan_protocol::{
    command::{Command, CommandResponse},
    message::Message,
};

use crate::{connection::Connection, error::YiZhanResult, serve::Serve};

pub(crate) struct YiZhanServer<S> {
    pub(crate) serve: S,
}

impl<S: Serve> YiZhanServer<S> {
    pub(crate) fn new(serve: S) -> Self {
        Self { serve }
    }
}

#[async_trait]
impl<S: Serve + Send + Sync> Connection for YiZhanServer<S> {
    async fn run(&self, sender: Sender<Message>) -> YiZhanResult<Message> {
        self.serve.run(sender).await
    }

    async fn request(&self, cmd: Command) -> YiZhanResult<CommandResponse> {
        self.serve.request(cmd).await
    }

    async fn send(&self, client_id: String, message: &Message) -> YiZhanResult<()> {
        self.serve.send(client_id, message).await
    }
}
