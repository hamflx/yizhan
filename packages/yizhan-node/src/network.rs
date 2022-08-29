use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{info, warn};
use nanoid::nanoid;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{oneshot, Mutex};
use tokio::{select, spawn};
use yizhan_protocol::command::CommandResponse;
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

        spawn({
            let conn = self.connection.clone();
            let close_sender = close_sender.clone();
            async move {
                let _ = conn.run(msg_tx).await;
                let _ = close_sender.send(());
            }
        });

        let command_map: Arc<Mutex<HashMap<String, oneshot::Sender<String>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        spawn({
            let conn = self.connection.clone();
            let command_map = command_map.clone();
            async move {
                while let Some(cmd) = cmd_rx.recv().await {
                    info!("Got command: {:?}", cmd);
                    let cmd_id = nanoid!();
                    match conn
                        .send(
                            String::new(), // todo 完善 client_id 的逻辑。
                            &Message::Command(cmd_id.clone(), cmd),
                        )
                        .await
                    {
                        Ok(_) => {
                            let mut lock = command_map.lock().await;
                            let (sender, receiver) = oneshot::channel();
                            lock.insert(cmd_id, sender);
                            receiver.await.unwrap();
                        }
                        Err(err) => warn!("Failed to send packet: {:?}", err),
                    }
                }

                let _ = close_sender.send(());
            }
        });

        spawn({
            let conn = self.connection.clone();
            let command_map = command_map.clone();
            async move {
                while let Some(msg) = msg_rx.recv().await {
                    info!("Got message: {:?}", msg);
                    match msg {
                        Message::Echo(conn_id) => {
                            info!("Client connected: {}", conn_id);
                        }
                        Message::Command(id, cmd) => match cmd {
                            command::Command::Run(program) => {
                                let mut child = Command::new(program.as_str());
                                match child.output() {
                                    Ok(output) => {
                                        info!("Sending response to peer");
                                        conn.send(
                                            String::new(), // todo 完善 client_id 的逻辑。
                                            &Message::CommandResponse(
                                                id,
                                                CommandResponse::Run(
                                                    std::str::from_utf8(output.stdout.as_slice())
                                                        .unwrap()
                                                        .to_string(),
                                                ),
                                            ),
                                        )
                                        .await
                                        .unwrap();
                                        info!("Response sent");
                                    }
                                    Err(err) => {
                                        warn!("Failed to read stdout: {:?}", err)
                                    }
                                }
                            }
                            _ => {
                                warn!("No command");
                            }
                        },
                        Message::CommandResponse(id, CommandResponse::Run(response)) => {
                            info!("Resolving command response.");
                            let mut lock = command_map.lock().await;
                            match lock.remove(&id) {
                                Some(sender) => {
                                    sender.send(response).unwrap();
                                }
                                _ => {}
                            }
                        }
                        msg => {
                            warn!("Unrecognized message: {:?}", msg);
                        }
                    }
                }
                info!("Message receiving task ended");
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
