use std::sync::Arc;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{info, warn};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::{select, spawn};

use crate::client::YiZhanClient;
use crate::connection::Connection;
use crate::console::Console;
use crate::error::YiZhanResult;
use crate::{serve::Serve, server::YiZhanServer};

pub(crate) struct YiZhanNetwork<Conn> {
    connection: Conn,
    consoles: Arc<Vec<Box<dyn Console>>>,
    config_channel_tx: Sender<YiZhanResult<()>>,
    config_channel_rx: Receiver<YiZhanResult<()>>,
}

impl<Conn: Connection> YiZhanNetwork<Conn> {
    pub(crate) fn new(connection: Conn) -> Self {
        let (config_channel_tx, config_channel_rx) = channel(40960);
        Self {
            connection,
            consoles: Arc::new(Vec::new()),
            config_channel_tx,
            config_channel_rx,
        }
    }

    pub(crate) async fn run(&mut self) -> YiZhanResult<()> {
        let (cmd_tx, cmd_rx) = channel(40960);
        let console = self.consoles.clone();
        spawn(async move {
            let mut stream = FuturesUnordered::new();

            loop {
                for con in console.iter() {
                    stream.push(con.run(cmd_tx.clone()));
                }
                while let Some(_) = stream.next().await {
                    info!("Got item from stream");
                }
                info!("Stream is empty");
            }
        });

        // let () = channel(40960);
        // spawn(async move {});

        select! {
            config_info = self.config_channel_rx.recv() => {

            }
        }
        // self.connection.run().await

        Ok(())
    }

    pub(crate) fn add_console(&self) {
        self.config_channel_tx.send(Ok(()));
    }

    pub(crate) fn add_connection() {}
}
