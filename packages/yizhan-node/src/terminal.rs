use std::{io::stdin, sync::Arc, thread::spawn};

use async_trait::async_trait;
use futures::executor::block_on;
use tokio::sync::{broadcast::Receiver, mpsc::Sender};
use tracing::{info, warn};

use crate::{
    commands::{parse_user_command, RequestCommand},
    console::Console,
    context::YiZhanContext,
    error::YiZhanResult,
};

pub(crate) struct Terminal {}

#[async_trait]
impl Console for Terminal {
    async fn run(
        &self,
        ctx: Arc<YiZhanContext>,
        sender: Sender<RequestCommand>,
        mut shut_rx: Receiver<()>,
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
                        block_on(sender.send(command))?;
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

impl Terminal {
    pub(crate) fn new() -> Self {
        Self {}
    }
}
