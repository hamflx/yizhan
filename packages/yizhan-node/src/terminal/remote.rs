use std::sync::Arc;

use async_trait::async_trait;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufStream},
    net::{TcpListener, TcpStream},
    select, spawn,
    sync::{broadcast, mpsc, oneshot},
};
use tracing::{info, warn};
use yizhan_common::error::YiZhanResult;
use yizhan_protocol::command::UserCommandResult;

use crate::{
    commands::{parse_user_command, ParseCommandResult, RequestCommand},
    console::Console,
    context::YiZhanContext,
    plugins::PluginManagement,
    terminal::show_response,
};

pub(crate) struct RemoteTerminal {}

impl RemoteTerminal {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Console for RemoteTerminal {
    async fn run(
        &self,
        ctx: Arc<YiZhanContext>,
        plugins: Arc<PluginManagement>,
        cmd_tx: mpsc::Sender<(RequestCommand, oneshot::Sender<UserCommandResult>)>,
        mut shut_rx: broadcast::Receiver<()>,
    ) -> YiZhanResult<()> {
        spawn({
            async move {
                let listener = match TcpListener::bind("127.0.0.1:3778").await {
                    Ok(listener) => listener,
                    Err(err) => return warn!("RemoteTerminal bind error: {:?}", err),
                };
                loop {
                    match select! {
                        r = listener.accept() => r,
                        _ = shut_rx.recv() => break,
                    } {
                        Ok((client, addr)) => {
                            info!("Remote terminal connected: {:?}", addr);
                            spawn({
                                let shut_rx = shut_rx.resubscribe();
                                let cmd_tx = cmd_tx.clone();
                                let ctx = ctx.clone();
                                let plugins = plugins.clone();
                                async move {
                                    if let Err(err) = handle_terminal_client(
                                        ctx.clone(),
                                        &plugins,
                                        client,
                                        cmd_tx,
                                        shut_rx,
                                    )
                                    .await
                                    {
                                        warn!("RemoteTerminal task error: {:?}", err);
                                    }
                                }
                            });
                        }
                        Err(err) => break warn!("RemoteTerminal accept error: {:?}", err),
                    }
                }
            }
        })
        .await?;

        Ok(())
    }
}

async fn handle_terminal_client(
    ctx: Arc<YiZhanContext>,
    plugins: &PluginManagement,
    stream: TcpStream,
    cmd_tx: mpsc::Sender<(RequestCommand, oneshot::Sender<UserCommandResult>)>,
    mut shut_rx: broadcast::Receiver<()>,
) -> YiZhanResult<()> {
    let mut stream = BufStream::new(stream);
    let mut line = String::new();
    loop {
        line.clear();

        let n = select! {
            _ = shut_rx.recv() => break,
            n = stream.read_line(&mut line) => n?
        };
        info!("RemoteTerminal read {} bytes", n);

        if n == 0 {
            warn!("handle_terminal_client eof");
            return Err(anyhow::anyhow!("handle_terminal_client eof"));
        }

        let response = match parse_user_command(&ctx, line.trim()) {
            Ok(command) => {
                let request = match command {
                    ParseCommandResult::Ok(command) => Ok(command),
                    ParseCommandResult::Unrecognized(args) => {
                        let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                        let args = args.as_slice();
                        let plugins = plugins.plugins.lock().await;
                        let mut parsed_command: Option<RequestCommand> = None;
                        for plugin in plugins.iter() {
                            if let Some((target, command)) = plugin.parse_command(args) {
                                parsed_command = Some(RequestCommand(target, command));
                                break;
                            }
                        }
                        match parsed_command {
                            Some(c) => Ok(c),
                            None => Err(anyhow::anyhow!("Unrecognized command: {:?}", args)),
                        }
                    }
                };
                match request {
                    Ok(request) => {
                        let (tx, rx) = oneshot::channel();
                        cmd_tx.send((request, tx)).await?;
                        let response = rx.await?;
                        show_response(response, plugins).await
                    }
                    Err(err) => format!("Err: {:?}", err),
                }
            }
            Err(err) => format!("Parse command error: {:?}\n", err),
        };

        info!("Sending response to remote terminal: {}", response);
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;
    }

    info!("End of handle_terminal_client");

    Ok(())
}
