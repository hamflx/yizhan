use std::str::FromStr;

use async_trait::async_trait;
use log::{info, warn};
use tokio::{
    io::{stdin, AsyncReadExt},
    sync::mpsc::Sender,
};
use yizhan_protocol::command::Command;

use crate::{command::RequestCommand, console::Console, error::YiZhanResult};

pub(crate) struct Terminal {}

#[async_trait]
impl Console for Terminal {
    async fn run(&self, sender: Sender<RequestCommand>) -> YiZhanResult<()> {
        let mut stdin = stdin();
        let mut buffer = [0; 4096];
        let mut line = String::new();

        loop {
            info!("Waiting for user input ...");
            let size = stdin.read(&mut buffer).await?;
            if size == 0 {
                return Err(anyhow::anyhow!("End of input"));
            }

            line.push_str(std::str::from_utf8(&buffer[..size])?);
            if line.is_empty() {
                continue;
            }

            if let Some(index) = line.chars().position(|c| c == '\n') {
                let current_line = line[..index].to_string();
                line = line[index + 1..].to_string();
                info!("Got line: {}", current_line);

                match Command::from_str(current_line.trim()) {
                    Ok(command) => {
                        info!("Send command to network.rs: {:?}", command);
                        sender.send(RequestCommand(None, command)).await?;
                    }
                    Err(err) => warn!("Parse command error: {:?}", err),
                }
            }
        }
    }
}

impl Terminal {
    pub(crate) fn new() -> Self {
        Self {}
    }
}
