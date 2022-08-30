use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{info, warn};
use nanoid::nanoid;
use tokio::sync::mpsc::channel;
use tokio::sync::{oneshot, Mutex};
use tokio::time::timeout;
use tokio::{join, spawn};
use yizhan_protocol::command::UserCommandResponse;
use yizhan_protocol::{command, message::Message};

use crate::command::RequestCommand;
use crate::connection::Connection;
use crate::console::Console;
use crate::error::YiZhanResult;

type CommandRegistry = Arc<Mutex<HashMap<String, oneshot::Sender<String>>>>;

pub(crate) struct YiZhanNetwork<Conn> {
    name: String,
    connection: Arc<Conn>,
    consoles: Arc<Mutex<Vec<Box<dyn Console>>>>,
}

impl<Conn: Connection + Send + Sync + 'static> YiZhanNetwork<Conn> {
    pub(crate) fn new(connection: Conn, name: String) -> Self {
        Self {
            name,
            connection: Arc::new(connection),
            consoles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) async fn run(self) -> YiZhanResult<()> {
        let (close_sender, mut close_receiver) = channel(10);

        let (cmd_tx, mut cmd_rx) = channel(40960);
        let (msg_tx, mut msg_rx) = channel(40960);

        let console_task = spawn({
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

                close_sender.send(()).await.unwrap();
                // }
            }
        });

        let connection_task = spawn({
            let conn = self.connection.clone();
            let close_sender = close_sender.clone();
            let name = self.name.clone();
            async move {
                match conn.run(&name, msg_tx).await {
                    Ok(_) => {}
                    Err(err) => warn!("Connection closed: {:?}", err),
                }
                close_sender.send(()).await.unwrap();
            }
        });

        let command_map: CommandRegistry = Arc::new(Mutex::new(HashMap::new()));
        let cmd_task = spawn({
            let conn = self.connection.clone();
            let command_map = command_map.clone();
            async move {
                while let Some(RequestCommand(node_id, cmd)) = cmd_rx.recv().await {
                    info!("Got command: {:?}", cmd);
                    let cmd_id = nanoid!();
                    let mut node_id_list = node_id.map(|id| vec![id]).unwrap_or_default();
                    if node_id_list.is_empty() {
                        node_id_list.extend(conn.get_peers().await.unwrap());
                    }
                    info!("Peer client_id_list: {:?}", node_id_list);
                    for node_id in node_id_list {
                        match conn
                            .send(
                                node_id,
                                &Message::CommandRequest(None, cmd_id.clone(), cmd.clone()),
                            )
                            .await
                        {
                            Ok(_) => {
                                request(&command_map, cmd_id.clone()).await;
                            }
                            Err(err) => warn!("Failed to send packet: {:?}", err),
                        }
                    }
                }

                info!("End of read command");
            }
        });

        let msg_task = spawn({
            let close_sender = close_sender.clone();
            let conn = self.connection.clone();
            let command_map = command_map.clone();
            async move {
                while let Some(msg) = msg_rx.recv().await {
                    info!("Got message: {:?}", msg);
                    match msg {
                        Message::Echo(conn_id) => {
                            info!("Client connected: {}", conn_id);
                        }
                        Message::CommandRequest(node_id, cmd_id, cmd) => match cmd {
                            command::UserCommand::Run(cmd_node_id, program) => {
                                let mut child = Command::new(program.as_str());
                                match child.output() {
                                    Ok(output) => {
                                        let mut node_id_list = node_id
                                            .or(cmd_node_id)
                                            .map(|id| vec![id])
                                            .unwrap_or_default();
                                        if node_id_list.is_empty() {
                                            node_id_list.extend(conn.get_peers().await.unwrap());
                                        }
                                        for node_id in node_id_list {
                                            info!("Sending response to peer {:?}", node_id);
                                            conn.send(
                                                node_id.clone(),
                                                &Message::CommandResponse(
                                                    Some(node_id.clone()),
                                                    cmd_id.clone(),
                                                    UserCommandResponse::Run(
                                                        std::str::from_utf8(
                                                            output.stdout.as_slice(),
                                                        )
                                                        .unwrap()
                                                        .to_string(),
                                                    ),
                                                ),
                                            )
                                            .await
                                            .unwrap();
                                            info!("Response sent");
                                        }
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
                        Message::CommandResponse(
                            _node_id,
                            cmd_id,
                            UserCommandResponse::Run(response),
                        ) => {
                            info!("Resolving command response.");
                            let mut lock = command_map.lock().await;
                            info!("Got command_map lock");
                            match lock.remove(&cmd_id) {
                                Some(sender) => {
                                    info!("Sending done signal");
                                    sender.send(response).unwrap();
                                }
                                _ => {
                                    info!("No command:{} found in command_map", cmd_id);
                                }
                            }
                        }
                    }
                }
                info!("Message receiving task ended");
                close_sender.send(()).await.unwrap();
            }
        });

        let (console_result, connection_result, cmd_result, msg_result) =
            join!(console_task, connection_task, cmd_task, msg_task);
        console_result?;
        connection_result?;
        cmd_result?;
        msg_result?;
        info!("Program shutdown.");

        Ok(())
    }

    pub(crate) async fn add_console(&mut self, console: Box<dyn Console>) {
        self.consoles.lock().await.push(console);
    }
}

async fn request(command_registry: &CommandRegistry, cmd_id: String) {
    let receiver = {
        let mut lock = command_registry.lock().await;
        let (sender, receiver) = oneshot::channel();
        lock.insert(cmd_id.clone(), sender);

        receiver
    };

    if let Err(err) = timeout(Duration::from_secs(1), receiver).await {
        warn!("Timed out: {:?}", err);
    } else {
        info!("Receiver done");
    }
}
