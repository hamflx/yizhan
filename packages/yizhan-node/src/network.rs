use std::sync::Arc;

use tokio::spawn;

use crate::client::YiZhanClient;
use crate::error::YiZhanResult;
use crate::{serve::Serve, server::YiZhanServer};

pub(crate) struct YiZhanNetwork<S: Send + Sync> {
    server: YiZhanServer<S>,
    client: YiZhanClient,
}

impl<S: Serve + Send + Sync> YiZhanNetwork<S> {
    pub(crate) fn new(server: YiZhanServer<S>, client: YiZhanClient) -> Self {
        Self { server, client }
    }

    pub(crate) async fn run(self) -> YiZhanResult<()> {
        let network = Arc::new(self);
        spawn({
            let network = network.clone();
            async {
                let _ = network.server.run().await;
            }
        });
        spawn(async { self.client.run().await });

        Ok(())
    }
}
