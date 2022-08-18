use tokio::{
    io::{stdin, AsyncReadExt},
    net::TcpStream,
    select,
};
use yizhan_protocol::message::Message;

use crate::{console::Console, error::YiZhanResult};

pub(crate) struct YiZhanClient {
    consoles: Vec<Box<dyn Console>>,
}

impl YiZhanClient {
    pub fn new() -> Self {
        Self {
            consoles: Vec::new(),
        }
    }

    pub(crate) async fn run(&self) -> YiZhanResult<()> {
        let stream = TcpStream::connect("127.0.0.1:3777").await?;

        loop {
            select! {
              cmd_res = self.read_command() => {
                let command = cmd_res?;
              }
              read_res = stream.readable() => {
                let message = read_res?;
              }
            }
        }
    }

    pub(crate) fn add_console(&mut self, console: Box<dyn Console>) {
        self.consoles.push(console);
    }

    async fn read_command(&self) -> YiZhanResult<Message> {
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
            }
            println!()
        }
    }
}
