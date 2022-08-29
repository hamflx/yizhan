use std::process::Command;
use std::sync::Arc;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{info, warn};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{oneshot, Mutex};
use tokio::{select, spawn};
use yizhan_protocol::{command, message::Message};

use crate::client::YiZhanClient;
use crate::connection::Connection;
use crate::console::Console;
use crate::error::YiZhanResult;
use crate::{serve::Serve, server::YiZhanServer};

pub(crate) struct YiZhanNetwork<Conn> {
    connection: Arc<Conn>,
    consoles: Arc<Mutex<Vec<Box<dyn Console>>>>,
    config_channel_tx: Sender<YiZhanResult<()>>,
    config_channel_rx: Receiver<YiZhanResult<()>>,
}

impl<Conn: Connection + Send + Sync + 'static> YiZhanNetwork<Conn> {
    pub(crate) fn new(connection: Conn) -> Self {
        let (config_channel_tx, config_channel_rx) = channel(40960);
        Self {
            connection: Arc::new(connection),
            consoles: Arc::new(Mutex::new(Vec::new())),
            config_channel_tx,
            config_channel_rx,
        }
    }

    pub(crate) async fn run(mut self) -> YiZhanResult<()> {
        let (close_sender, mut close_receiver) = channel(1);

        let (cmd_tx, mut cmd_rx) = channel(40960);
        let (msg_tx, mut msg_rx) = channel(40960);

        spawn({
            let consoles = self.consoles.clone();
            let close_sender = close_sender.clone();
            async move {
                let console_list = consoles.lock().await;
                let mut stream = FuturesUnordered::new();

                // loop {
                info!("Console length: {}", console_list.len());
                for con in console_list.iter() {
                    stream.push(con.run(cmd_tx.clone()));
                }
                while let Some(_) = stream.next().await {
                    info!("Got item from stream");
                }
                info!("Stream is empty");

                let _ = close_sender.send(());
                // }
            }
        });

        // let () = channel(40960);
        spawn({
            let conn = self.connection.clone();
            let close_sender = close_sender.clone();
            async move {
                let _ = conn.run(msg_tx).await;
                let _ = close_sender.send(());
            }
        });

        spawn({
            let conn = self.connection.clone();
            async move {
                while let Some(cmd) = cmd_rx.recv().await {
                    info!("Got command: {:?}", cmd);
                    let _ = conn.send(&Message::Command(cmd)).await;
                }

                let _ = close_sender.send(());
            }
        });

        spawn({
            async move {
                while let Some(msg) = msg_rx.recv().await {
                    info!("Got message: {:?}", msg);
                    match msg {
                        Message::Command(cmd) => match cmd {
                            command::Command::Run(program) => {
                                let mut child = Command::new(program.as_str());
                                match child.output() {
                                    Ok(output) => {
                                        info!(
                                            "Program [{}] output: {}",
                                            program,
                                            std::str::from_utf8(output.stdout.as_slice()).unwrap()
                                        )
                                    }
                                    Err(err) => {
                                        warn!("Failed to read stdout: {:?}", err)
                                    }
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        });

        close_receiver.recv().await;

        Ok(())
    }

    pub(crate) async fn add_console(&mut self, console: Box<dyn Console>) {
        self.consoles.lock().await.push(console);
    }

    pub(crate) fn add_connection() {}
}
