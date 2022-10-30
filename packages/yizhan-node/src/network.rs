use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use nanoid::nanoid;
use tokio::sync::mpsc::channel;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio::time::timeout;
use tokio::{select, spawn};
use tracing::{info, span, warn, Instrument, Level};
use yizhan_common::error::YiZhanResult;
use yizhan_plugin::Plugin;
use yizhan_protocol::command::{UserCommand, UserCommandResponse, UserCommandResult};
use yizhan_protocol::message::Message;
use yizhan_protocol::version::VersionInfo;

use crate::commands::common::send_response;
use crate::commands::get::do_get_command;
use crate::commands::run::do_run_command;
use crate::commands::uninstall::do_uninstall_command;
use crate::commands::update::do_update_command;
use crate::commands::RequestCommand;
use crate::config::YiZhanNodeConfig;
use crate::connection::Connection;
use crate::console::Console;
use crate::context::YiZhanContext;
use crate::plugins::PluginManagement;

pub(crate) type CommandRegistry = Arc<Mutex<HashMap<String, oneshot::Sender<UserCommandResult>>>>;
pub(crate) type ShutdownHooks = Arc<Mutex<Vec<Box<dyn FnOnce() + Send>>>>;

pub(crate) struct YiZhanNetwork<Conn> {
    connection: Arc<Conn>,
    consoles: Arc<Mutex<Vec<Box<dyn Console>>>>,
    context: Arc<YiZhanContext>,
    plugins: PluginManagement,
}

impl<Conn: Connection + Send + Sync + 'static> YiZhanNetwork<Conn> {
    pub(crate) fn new(
        connection: Conn,
        name: String,
        version: VersionInfo,
        server_mode: bool,
        config: YiZhanNodeConfig,
    ) -> Self {
        Self {
            connection: Arc::new(connection),
            consoles: Arc::new(Mutex::new(Vec::new())),
            context: Arc::new(YiZhanContext {
                name,
                version,
                server_mode,
                config,
            }),
            plugins: PluginManagement::new(),
        }
    }

    pub(crate) async fn run(self) -> YiZhanResult<()> {
        let shutdown_hooks: ShutdownHooks = Arc::new(Mutex::new(Vec::new()));

        run_tasks(
            self.connection,
            self.context,
            self.consoles,
            self.plugins,
            shutdown_hooks.clone(),
        )
        .await;

        let shutdown_hooks = {
            let mut shutdown_hooks = shutdown_hooks.lock().await;
            shutdown_hooks.drain(..).collect::<Vec<_>>()
        };
        for hook in shutdown_hooks {
            hook();
        }

        Ok(())
    }

    pub(crate) async fn add_console(&mut self, console: Box<dyn Console>) {
        self.consoles.lock().await.push(console);
    }

    pub(crate) async fn add_plugin(&self, plugin: Box<dyn Plugin>) {
        self.plugins.add_plugin(plugin).await;
    }
}

pub(crate) async fn run_tasks<Conn: Connection + Send + Sync + 'static>(
    connection: Arc<Conn>,
    context: Arc<YiZhanContext>,
    consoles: Arc<Mutex<Vec<Box<dyn Console>>>>,
    plugins: PluginManagement,
    shutdown_hooks: ShutdownHooks,
) {
    // todo 关闭所有的 task。
    let (shut_tx, mut shut_rx) = broadcast::channel(10);

    let (cmd_tx, mut cmd_rx) = channel(40960);
    let (msg_tx, mut msg_rx) = channel(40960);

    let plugins = Arc::new(plugins);

    let console_task = spawn({
        let ctx = context.clone();
        let consoles = consoles.clone();
        let shut_tx = shut_tx.clone();
        let plugins = plugins.clone();
        async move {
            let console_list = consoles.lock().await;
            let mut stream = FuturesUnordered::new();

            info!("Console length: {}", console_list.len());
            for con in console_list.iter() {
                stream.push(con.run(
                    ctx.clone(),
                    plugins.clone(),
                    cmd_tx.clone(),
                    shut_tx.subscribe(),
                ));
            }

            while stream.next().await.is_some() {}
            info!("End of console task");
        }
        .instrument(span!(Level::TRACE, "console task"))
    });

    let connection_task = spawn({
        let ctx = context.clone();
        let conn = connection.clone();
        let shut_tx = shut_tx.clone();
        async move {
            if let Err(err) = conn.run(ctx, msg_tx, shut_tx.subscribe()).await {
                warn!("Connection closed: {:?}", err);
            }
            if let Err(err) = shut_tx.send(()) {
                warn!("Send failed: {:?}", err);
            }
            info!("End of connection task");
        }
        .instrument(span!(Level::TRACE, "connection task"))
    });

    let command_map: CommandRegistry = Arc::new(Mutex::new(HashMap::new()));
    let cmd_task = spawn({
        let ctx = context.clone();
        let conn = connection.clone();
        let command_map = command_map.clone();
        let mut shut_rx = shut_tx.subscribe();
        async move {
            while let Some((RequestCommand(target_node_id, cmd), resp_tx)) = select! {
                _ = shut_rx.recv() => None,
                r = cmd_rx.recv() => r,
            } {
                let cmd_id = nanoid!();
                let peers = conn.get_peers().await.unwrap();
                let send_target = target_node_id
                    .as_ref()
                    .filter(|s| peers.iter().any(|n| n.id == **s) && **s != ctx.name)
                    // todo 因为这块逻辑暂时只有客户端有，而目前客户端目前又仅有一个连接，所以，此处取第一个是可行的。
                    .or_else(|| peers.first().map(|s| &s.id));

                if let Some(send_target) = send_target {
                    info!(
                        "Sending command {} to: {} with target: {:?}",
                        cmd_id, send_target, target_node_id
                    );
                    match conn
                        .send(
                            send_target.clone(),
                            Message::CommandRequest {
                                target: target_node_id,
                                source: None,
                                cmd_id: cmd_id.clone(),
                                cmd: cmd.clone(),
                            },
                        )
                        .await
                    {
                        Ok(_) => match request_cmd(&command_map, cmd_id.clone()).await {
                            Ok(response) => {
                                if let Err(err) = resp_tx.send(response) {
                                    warn!("Send response error: {:?}", err);
                                }
                            }
                            Err(err) => warn!("Wait command response error: {:?}", err),
                        },
                        Err(err) => warn!("Failed to send packet: {:?}", err),
                    }
                } else {
                    warn!("No send target, ignored");
                }
            }

            info!("End of read command");
        }
        .instrument(span!(Level::TRACE, "command task"))
    });

    let msg_task = spawn({
        let ctx = context.clone();
        let conn = connection.clone();
        let command_map = command_map.clone();
        let shut_tx = shut_tx.clone();
        let shutdown_hooks = shutdown_hooks.clone();
        async move {
            while let Some((src_node_id, msg)) = select! {
                r = msg_rx.recv() => r,
                _ = shut_rx.recv() => None,
            } {
                match msg {
                    Message::Echo(node_info) => {
                        info!("Client connected: {:?}", node_info);
                    }
                    Message::CommandRequest {
                        target,
                        source,
                        cmd,
                        cmd_id,
                    } => {
                        info!("Got command sending to {:?}", target);

                        let is_self_node = target.as_ref() == Some(&ctx.name);
                        let should_forward = !matches!(cmd, UserCommand::Ls);
                        // todo 这里 Ls 命令通过广播又发回自己后，回导致后续命令得不到输出结果，暂时不转发该命令。
                        if should_forward {
                            forward_message(
                                is_self_node,
                                target.clone(),
                                &conn,
                                |node_id| Message::CommandRequest {
                                    target: Some(node_id.to_string()),
                                    source: Some(src_node_id.clone()),
                                    cmd_id: cmd_id.to_string(),
                                    cmd: cmd.clone(),
                                },
                                &ctx,
                            )
                            .await;
                        }

                        if is_self_node || ctx.server_mode && target.is_none() {
                            let src_node_id = match (ctx.server_mode, source) {
                                (true, _) => src_node_id.clone(),
                                (false, None) => {
                                    warn!("No source id");
                                    continue;
                                }
                                (false, Some(node_id)) => node_id,
                            };
                            handle_command(
                                &plugins,
                                cmd.clone(),
                                &shut_tx,
                                &ctx,
                                cmd_id.clone(),
                                &*conn,
                                src_node_id,
                                &shutdown_hooks,
                            )
                            .await;
                        }
                    }
                    Message::CommandResponse(target_node_id, cmd_id, response) => {
                        info!(
                            "Received command {} response to {:?}",
                            cmd_id, target_node_id
                        );
                        let is_self_node = target_node_id.as_ref() == Some(&ctx.name);
                        forward_message(
                            is_self_node,
                            target_node_id.clone(),
                            &conn,
                            |node_id| {
                                Message::CommandResponse(
                                    Some(node_id.to_string()),
                                    cmd_id.to_string(),
                                    response.clone(),
                                )
                            },
                            &ctx,
                        )
                        .await;

                        if target_node_id.as_ref() == Some(&ctx.name) {
                            response_cmd(&command_map, &cmd_id, response.clone()).await;
                        }
                    }
                    _ => {}
                }
            }
            info!("End of message task");
            shut_tx.send(()).unwrap();
        }
        .instrument(span!(Level::TRACE, "message task"))
    });

    let _ = console_task.await;
    let _ = connection_task.await;
    let _ = cmd_task.await;
    let _ = msg_task.await;

    let _ = connection.flush().await;

    info!("Program shutdown.");
}

async fn forward_message<Conn: Connection, F: Fn(&str) -> Message>(
    is_self_node: bool,
    target_node_id: Option<String>,
    conn: &Arc<Conn>,
    build_msg: F,
    ctx: &Arc<YiZhanContext>,
) {
    // forward
    if !is_self_node {
        if let Some(node_id) = &target_node_id {
            info!("Forwarding message to: {}", node_id);
            if let Err(err) = conn
                .send(node_id.clone(), build_msg(node_id.as_str()))
                .await
            {
                warn!("forward_message error: {:?}", err);
            } else {
                info!("forward message sent");
            }
        }
    }

    // broadcast
    if target_node_id.is_none() && ctx.server_mode {
        match conn.get_peers().await {
            Ok(peers) => {
                for node_info in peers {
                    info!("Forward message to peer: {:?}", node_info);
                    if let Err(err) = conn
                        .send(node_info.id.clone(), build_msg(node_info.id.as_str()))
                        .await
                    {
                        warn!("Forward error: {:?}", err);
                    } else {
                        info!("broadcast sent");
                    }
                }
            }
            Err(err) => warn!("Error: {:?}", err),
        }
    }
}

async fn handle_command<Conn: Connection>(
    plugins: &PluginManagement,
    cmd: UserCommand,
    shut_tx: &broadcast::Sender<()>,
    ctx: &YiZhanContext,
    cmd_id: String,
    conn: &Conn,
    src_node_id: String,
    shutdown_hooks: &ShutdownHooks,
) {
    match cmd {
        UserCommand::Halt => {
            shut_tx.send(()).unwrap();
        }
        UserCommand::Run(program, args) => {
            do_run_command(ctx, Some(src_node_id), cmd_id, conn, program, args).await;
        }
        UserCommand::Update(version, platform, sha256, bytes) => {
            do_update_command(
                ctx,
                platform.as_str(),
                Some(src_node_id),
                cmd_id,
                conn,
                version,
                sha256,
                bytes,
                shut_tx,
                shutdown_hooks,
            )
            .await;
        }
        UserCommand::Ls => {
            let response = match conn.get_peers().await {
                Ok(peers) => UserCommandResult::Ok(UserCommandResponse::Ls(peers)),
                Err(err) => UserCommandResult::Err(format!("Error: {:?}", err)),
            };
            send_response(Some(src_node_id), conn, ctx, cmd_id, response).await;
        }
        UserCommand::PluginCommand(group_id, content) => {
            let plugins = plugins.plugins.lock().await;
            for plugin in plugins.iter() {
                if let Some(response) = plugin.execute_command(group_id.as_str(), content.as_str())
                {
                    send_response(
                        Some(src_node_id.clone()),
                        conn,
                        ctx,
                        cmd_id.clone(),
                        response,
                    )
                    .await;
                    break;
                }
            }
        }
        UserCommand::Get(path) => {
            do_get_command(ctx, Some(src_node_id), cmd_id, conn, path).await;
        }
        UserCommand::Uninstall => {
            do_uninstall_command(ctx, Some(src_node_id), cmd_id, conn).await;
        }
    }
}

async fn request_cmd(
    command_registry: &CommandRegistry,
    cmd_id: String,
) -> YiZhanResult<UserCommandResult> {
    let receiver = {
        let mut lock = command_registry.lock().await;
        let (sender, receiver) = oneshot::channel();
        info!("Insert command {} waiting sender", cmd_id);
        lock.insert(cmd_id.clone(), sender);

        receiver
    };

    // todo 这个超时时间太僵硬了，理论上来讲，应该在收到响应数据包的时候，就重设等待的超时时间。
    // 不过目前这个似乎还做不了，因为现在是接收整个数据包的形式，应该优化成流式接收，先收头，后收体。
    Ok(match timeout(Duration::from_secs(15), receiver).await {
        Err(err) => UserCommandResult::Err(format!("Timed out: {:?}", err)),
        Ok(Err(err)) => UserCommandResult::Err(format!("Unknown error: {:?}", err)),
        Ok(res) => res?,
    })
}

async fn response_cmd(
    cmd_registry: &CommandRegistry,
    cmd_id: &String,
    response: UserCommandResult,
) {
    let entry = {
        info!("Resolving command {} response.", cmd_id);
        let mut lock = cmd_registry.lock().await;
        info!("Got command_map lock");
        lock.remove(cmd_id)
    };
    match entry {
        Some(sender) => {
            info!("Sending done signal");
            if let Err(err) = sender.send(response) {
                warn!("Error: {:?}", err);
            }
        }
        _ => {
            info!("No command:{} found in command_map", cmd_id);
        }
    }
}
