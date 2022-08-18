use async_trait::async_trait;
use tokio::{
    io::{stdin, AsyncReadExt},
    sync::mpsc::Sender,
};

use crate::{command::Command, console::Console, error::YiZhanResult};

pub(crate) struct Terminal {}

#[async_trait]
impl Console for Terminal {
    async fn run(&self, sender: Sender<Command>) -> YiZhanResult<()> {
        let mut stdin = stdin();
        let mut buffer = [0; 4096];
        let mut line = String::new();

        loop {
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

                println!("Got line: {}", current_line);
                sender.send(Command::Echo(current_line)).await?;
            }
        }
    }
}

impl Terminal {
    pub(crate) fn new() -> Self {
        Self {}
    }
}
