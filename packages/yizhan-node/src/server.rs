use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use yizhan_protocol::message::Message;

use crate::{connection::Connection, context::YiZhanContext, error::YiZhanResult, serve::Serve};

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
    async fn run(&self, ctx: Arc<YiZhanContext>, sender: Sender<Message>) -> YiZhanResult<Message> {
        self.serve.run(ctx, sender).await
    }

    async fn get_peers(&self) -> YiZhanResult<Vec<String>> {
        self.serve.get_peers().await
    }

    async fn send(&self, client_id: String, message: &Message) -> YiZhanResult<()> {
        self.serve.send(client_id, message).await
    }
}
