use std::{io::stdin, sync::Arc, thread::spawn};

use async_trait::async_trait;
use futures::executor::block_on;
use tokio::sync::{broadcast, mpsc, oneshot};
use tracing::{info, warn};
use yizhan_common::error::YiZhanResult;
use yizhan_protocol::command::UserCommandResult;

use crate::{
    commands::{parse_user_command, ParseCommandResult, RequestCommand},
    console::Console,
    context::YiZhanContext,
    plugins::PluginManagement,
};

pub(crate) struct LocalTerminal {}

#[async_trait]
impl Console for LocalTerminal {
    async fn run(
        &self,
        ctx: Arc<YiZhanContext>,
        plugins: Arc<PluginManagement>,
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

                let command = match parse_user_command(&ctx, line.trim()) {
                    Ok(result) => match result {
                        ParseCommandResult::Ok(command) => command,
                        ParseCommandResult::Unrecognized(args) => {
                            let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                            let args = args.as_slice();
                            let plugins = block_on(plugins.plugins.lock());
                            let mut parsed_command: Option<RequestCommand> = None;
                            for plugin in plugins.iter() {
                                if let Some((target, command)) = plugin.parse_command(args) {
                                    parsed_command = Some(RequestCommand(target, command));
                                    break;
                                }
                            }
                            match parsed_command {
                                Some(c) => c,
                                None => {
                                    warn!("Unrecognized command: {:?}", args);
                                    continue;
                                }
                            }
                        }
                    },
                    Err(err) => {
                        warn!("Parse command error: {:?}", err);
                        continue;
                    }
                };

                let (tx, rx) = oneshot::channel();
                block_on(sender.send((command, tx)))?;
                match block_on(rx) {
                    Ok(response) => info!("Response: {:?}", response),
                    Err(err) => warn!("Error: {:?}", err),
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
