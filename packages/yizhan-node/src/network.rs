use std::collections::HashMap;
use std::str::FromStr;
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
use yizhan_protocol::command::{UserCommand, UserCommandResponse};
use yizhan_protocol::message::Message;
use yizhan_protocol::version::VersionInfo;

use crate::commands::run::do_run_command;
use crate::commands::update::do_update_command;
use crate::commands::RequestCommand;
use crate::connection::Connection;
use crate::console::Console;
use crate::context::YiZhanContext;
use crate::error::YiZhanResult;

type CommandRegistry = Arc<Mutex<HashMap<String, oneshot::Sender<UserCommandResponse>>>>;

pub(crate) struct YiZhanNetwork<Conn> {
    connection: Arc<Conn>,
    consoles: Arc<Mutex<Vec<Box<dyn Console>>>>,
    context: Arc<YiZhanContext>,
}

impl<Conn: Connection + Send + Sync + 'static> YiZhanNetwork<Conn> {
    pub(crate) fn new(connection: Conn, name: String, version: &str) -> Self {
        Self {
            connection: Arc::new(connection),
            consoles: Arc::new(Mutex::new(Vec::new())),
            context: Arc::new(YiZhanContext {
                name,
                version: VersionInfo::from_str(version).unwrap(),
            }),
        }
    }

    pub(crate) async fn run(self) -> YiZhanResult<()> {
        // todo 关闭所有的 task。
        let (close_sender, _close_receiver) = channel(10);

        let (cmd_tx, mut cmd_rx) = channel(40960);
        let (msg_tx, mut msg_rx) = channel(40960);

        let console_task = spawn({
            let ctx = self.context.clone();
            let consoles = self.consoles.clone();
            let close_sender = close_sender.clone();
            async move {
                let console_list = consoles.lock().await;
                let mut stream = FuturesUnordered::new();

                // loop {
                info!("Console length: {}", console_list.len());
                for con in console_list.iter() {
                    stream.push(con.run(ctx.clone(), cmd_tx.clone()));
                }
                while stream.next().await.is_some() {
                    info!("Got item from stream");
                }
                info!("Stream is empty");

                close_sender.send(()).await.unwrap();
                // }
            }
        });

        let connection_task = spawn({
            let ctx = self.context.clone();
            let conn = self.connection.clone();
            let close_sender = close_sender.clone();
            async move {
                match conn.run(ctx, msg_tx).await {
                    Ok(_) => {}
                    Err(err) => warn!("Connection closed: {:?}", err),
                }
                close_sender.send(()).await.unwrap();
            }
        });

        let command_map: CommandRegistry = Arc::new(Mutex::new(HashMap::new()));
        let cmd_task = spawn({
            let ctx = self.context.clone();
            let conn = self.connection.clone();
            let command_map = command_map.clone();
            async move {
                while let Some(RequestCommand(node_id, cmd)) = cmd_rx.recv().await {
                    let cmd_id = nanoid!();
                    let mut node_id_list = node_id.map(|id| vec![id]).unwrap_or_default();
                    if node_id_list.is_empty() {
                        node_id_list.extend(conn.get_peers().await.unwrap());
                    }
                    info!("Peer client_id_list: {:?}", node_id_list);
                    for node_id in node_id_list {
                        if node_id != *ctx.name {
                            info!("Sending command to: {}", node_id);
                            match conn
                                .send(
                                    node_id,
                                    &Message::CommandRequest(None, cmd_id.clone(), cmd.clone()),
                                )
                                .await
                            {
                                Ok(_) => {
                                    request_cmd(&command_map, cmd_id.clone()).await;
                                }
                                Err(err) => warn!("Failed to send packet: {:?}", err),
                            }
                        }
                    }
                }

                info!("End of read command");
            }
        });

        let msg_task = spawn({
            let ctx = self.context.clone();
            let close_sender = close_sender.clone();
            let conn = self.connection.clone();
            let command_map = command_map.clone();
            async move {
                while let Some(msg) = msg_rx.recv().await {
                    match msg {
                        Message::Echo(conn_id) => {
                            info!("Client connected: {}", conn_id);
                        }
                        Message::CommandRequest(node_id, cmd_id, cmd) => match cmd {
                            UserCommand::Run(program) => {
                                do_run_command(ctx.name.as_str(), node_id, cmd_id, &*conn, program)
                                    .await;
                            }
                            UserCommand::Update(version, sha256, bytes) => {
                                do_update_command(
                                    ctx.name.as_str(),
                                    node_id,
                                    cmd_id,
                                    &*conn,
                                    version,
                                    sha256,
                                    bytes,
                                )
                                .await;
                            }
                        },
                        Message::CommandResponse(_node_id, cmd_id, response) => {
                            response_cmd(&command_map, &cmd_id, response).await;
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

async fn request_cmd(command_registry: &CommandRegistry, cmd_id: String) {
    let receiver = {
        let mut lock = command_registry.lock().await;
        let (sender, receiver) = oneshot::channel();
        lock.insert(cmd_id.clone(), sender);

        receiver
    };

    match timeout(Duration::from_secs(15), receiver).await {
        Err(err) => warn!("Timed out: {:?}", err),
        Ok(Err(err)) => warn!("Unknown error: {:?}", err),
        Ok(res) => info!("Received command response: {:?}", res),
    }
}

async fn response_cmd(
    cmd_registry: &CommandRegistry,
    cmd_id: &String,
    response: UserCommandResponse,
) {
    let entry = {
        info!("Resolving command response.");
        let mut lock = cmd_registry.lock().await;
        info!("Got command_map lock");
        lock.remove(cmd_id)
    };
    match entry {
        Some(sender) => {
            info!("Sending done signal");
            sender.send(response).unwrap();
        }
        _ => {
            info!("No command:{} found in command_map", cmd_id);
        }
    }
}
