use std::{io::stdin, sync::Arc, thread::spawn};

use async_trait::async_trait;
use futures::executor::block_on;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{info, warn};
use yizhan_protocol::command::UserCommandResult;

use crate::{
    commands::{parse_user_command, RequestCommand},
    console::Console,
    context::YiZhanContext,
    error::YiZhanResult,
};

pub(crate) struct LocalTerminal {}

#[async_trait]
impl Console for LocalTerminal {
    async fn run(
        &self,
        ctx: Arc<YiZhanContext>,
        sender: mpsc::Sender<(RequestCommand, oneshot::Sender<UserCommandResult>)>,
        mut shut_rx: broadcast::Receiver<()>,
    ) -> YiZhanResult<()> {
        spawn(move || {
            let stdin = stdin();

            loop {
                info!("Waiting for user input ...");
                let mut line = String::new();
                let size = stdin.read_line(&mut line)?;
                if size == 0 {
                    return Err(anyhow::anyhow!("End of input")) as YiZhanResult<()>;
                }

                match parse_user_command(&ctx, line.trim()) {
                    Ok(command) => {
                        let (tx, rx) = oneshot::channel();
                        block_on(sender.send((command, tx)))?;
                        match block_on(rx) {
                            Ok(response) => info!("Response: {:?}", response),
                            Err(err) => warn!("Error: {:?}", err),
                        }
                    }
                    Err(err) => warn!("Parse command error: {:?}", err),
                }
            }
        });

        shut_rx.recv().await?;
        // todo terminate thread

        Ok(())
    }
}

impl LocalTerminal {
    pub(crate) fn new() -> Self {
        Self {}
    }
}
