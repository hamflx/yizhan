use tokio::select;

use crate::client::YiZhanClient;
use crate::error::YiZhanResult;
use crate::{serve::Serve, server::YiZhanServer};

pub(crate) struct YiZhanNetwork<S> {
    server: YiZhanServer<S>,
    client: YiZhanClient,
}

impl<S: Serve> YiZhanNetwork<S> {
    pub(crate) fn new(server: YiZhanServer<S>, client: YiZhanClient) -> Self {
        Self { server, client }
    }

    pub(crate) async fn run(&self) -> YiZhanResult<()> {
        select! {
            server_res = self.server.run() => {
                server_res?;
            }
            client_res = self.client.run() => {
                client_res?;
            }
        }

        Ok(())
    }
}
