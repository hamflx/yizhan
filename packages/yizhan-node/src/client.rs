use tokio::{
    io::{stdin, AsyncReadExt},
    net::TcpStream,
    select,
    sync::mpsc::channel,
};
use yizhan_protocol::message::Message;

use crate::{console::Console, error::YiZhanResult};

pub(crate) struct YiZhanClient {
    console: Box<dyn Console>,
}

impl YiZhanClient {
    pub fn new<C: Console + 'static>(console: C) -> Self {
        Self {
            console: Box::new(console),
        }
    }

    pub(crate) async fn run(&self) -> YiZhanResult<()> {
        let stream = TcpStream::connect("127.0.0.1:3777").await?;

        let (cmd_tx, cmd_rx) = channel(40960);
        self.handle_remote_message(&stream);
        self.console.run(cmd_tx).await?;

        // let (msg_tx, msg_rx) = channel();

        // select! {
        //   cmd_res = self.handle_user_input(&stream, &cmd_tx, &msg_rx) => {
        //     cmd_res?;
        //   }
        //   read_res = self.handle_remote_message(&stream) => {
        //     read_res?;
        //   }
        // }

        Ok(())
    }

    async fn handle_remote_message(&self, stream: &TcpStream) -> YiZhanResult<()> {
        stream.readable().await?;
        let mut buffer = vec![0; 4096];
        // match stream.try_read(&mut buffer) {}
        Ok(())
    }

    fn create_command_from_input(&self, input: String) -> YiZhanResult<Message> {
        let input = input.trim();
        Ok(Message::Echo(input.to_string()))
    }

    async fn handle_user_input(&self, stream: &TcpStream) -> YiZhanResult<()> {
        let command = self.create_command_from_input(self.read_user_input().await?)?;
        stream.writable().await?;
        Ok(())
    }

    async fn read_user_input(&self) -> YiZhanResult<String> {
        Ok(String::new())
    }
}
