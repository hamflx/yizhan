use std::sync::Arc;

use async_trait::async_trait;
use futures::executor::block_on;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpListener, TcpStream},
    select, spawn,
    sync::{broadcast, mpsc},
};
use tracing::{info, warn};

use crate::{
    commands::{parse_user_command, RequestCommand},
    console::Console,
    context::YiZhanContext,
    error::YiZhanResult,
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
        sender: mpsc::Sender<RequestCommand>,
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
                                let sender = sender.clone();
                                let ctx = ctx.clone();
                                async move {
                                    if let Err(err) =
                                        handle_terminal_client(ctx.clone(), client, sender, shut_rx)
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
    stream: TcpStream,
    sender: mpsc::Sender<RequestCommand>,
    mut shut_rx: broadcast::Receiver<()>,
) -> YiZhanResult<()> {
    let mut stream = BufReader::new(stream);
    let mut line = String::new();
    loop {
        let n = select! {
            _ = shut_rx.recv() => break,
            n = stream.read_line(&mut line) => n?
        };
        info!("RemoteTerminal read {} bytes", n);

        if n == 0 {
            warn!("handle_terminal_client eof");
            return Err(anyhow::anyhow!("handle_terminal_client eof"));
        }

        // todo 怎么样把命令回显给客户端？
        match parse_user_command(&ctx, line.trim()) {
            Ok(command) => {
                block_on(sender.send(command))?;
            }
            Err(err) => warn!("Parse command error: {:?}", err),
        }
    }

    info!("End of handle_terminal_client");

    Ok(())
}
