use async_trait::async_trait;
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use yizhan_protocol::message::{Message, WELCOME_MESSAGE};

use crate::error::YiZhanResult;
use crate::serve::Serve;

pub(crate) struct TcpServe {}

#[async_trait]
impl Serve for TcpServe {
    async fn run(&self) -> YiZhanResult<()> {
        let listner = TcpListener::bind("127.0.0.1:3777").await?;
        loop {
            let (stream, _) = listner.accept().await?;
            spawn(handle_client(stream));
        }
    }
}

async fn handle_client(stream: TcpStream) -> YiZhanResult<()> {
    loop {
        stream.writable().await?;

        let welcome_message = Message::Echo(WELCOME_MESSAGE.to_string());
        stream.try_write(bincode::serialize(&welcome_message)?.as_slice())?;

        stream.readable().await?;
    }
}

unsafe impl Send for TcpServe {}

unsafe impl Sync for TcpServe {}
