use async_trait::async_trait;
use yizhan_protocol::message::Message;

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
    async fn run(&self) -> YiZhanResult<Message> {
        self.serve.run().await
    }
}
