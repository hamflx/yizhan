use log::warn;
use tokio::select;

use crate::client::YiZhanClient;
use crate::connection::Connection;
use crate::error::YiZhanResult;
use crate::{serve::Serve, server::YiZhanServer};

pub(crate) struct YiZhanNetwork<C: Connection> {
    connection: C,
}

impl<C: Connection> YiZhanNetwork<C> {
    pub(crate) fn new(connection: C) -> Self {
        Self { connection }
    }

    pub(crate) async fn run(&self) -> YiZhanResult<()> {
        self.connection.run().await
    }
}
