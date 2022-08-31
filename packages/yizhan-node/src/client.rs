use std::{io, sync::Arc};

use async_trait::async_trait;
use bincode::{config, decode_from_slice, encode_to_vec};
use log::info;
use tokio::{
    net::TcpStream,
    sync::{mpsc::Sender, Mutex},
};
use yizhan_protocol::message::Message;

use crate::{connection::Connection, context::YiZhanContext, error::YiZhanResult};

pub(crate) struct YiZhanClient {
    stream: TcpStream,
    peer_id: Mutex<Option<String>>,
}

impl YiZhanClient {
    pub(crate) async fn new() -> YiZhanResult<Self> {
        Ok(Self {
            stream: TcpStream::connect("127.0.0.1:3777").await?,
            peer_id: Mutex::new(None),
        })
    }

    async fn handle_remote_message(
        &self,
        stream: &TcpStream,
        buffer: &mut [u8],
        cached_size: &mut usize,
    ) -> YiZhanResult<Option<Message>> {
        let remains_buffer = &mut buffer[*cached_size..];
        if remains_buffer.is_empty() {
            return Err(anyhow::anyhow!("No enough space"));
        }

        let size = match stream.try_read(remains_buffer) {
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(err) => return Err(err.into()),
            Ok(0) => return Err(anyhow::anyhow!("End of stream.")),
            Ok(size) => size,
        };

        *cached_size += size;
        let packet = &buffer[..*cached_size];

        let message: Message = match decode_from_slice(packet, config::standard()) {
            Ok((msg, len)) => {
                *cached_size -= len;
                msg
            }
            Err(_) => return Ok(None),
        };

        Ok(Some(message))
    }
}

#[async_trait]
impl Connection for YiZhanClient {
    async fn run(&self, ctx: Arc<YiZhanContext>, sender: Sender<Message>) -> YiZhanResult<Message> {
        let mut buffer = vec![0; 40960];
        let mut cached_size = 0;

        loop {
            self.stream.readable().await?;

            if let Some(msg) = self
                .handle_remote_message(&self.stream, buffer.as_mut_slice(), &mut cached_size)
                .await?
            {
                match &msg {
                    Message::Echo(server_id) => {
                        info!("Sending echo");

                        let mut lock = self.peer_id.lock().await;
                        *lock = Some(server_id.clone());

                        self.stream.writable().await?;
                        let echo_packet = encode_to_vec(
                            &Message::Echo(ctx.name.to_string()),
                            config::standard(),
                        )?;
                        self.stream.try_write(echo_packet.as_slice())?;
                    }
                    _ => {
                        info!("Not implemented message");
                    }
                }
                sender.send(msg).await?;
            }
        }
    }

    async fn get_peers(&self) -> YiZhanResult<Vec<String>> {
        let lock = self.peer_id.lock().await;
        Ok(lock.as_ref().map(|id| vec![id.clone()]).unwrap_or_default())
    }

    async fn send(&self, _client_id: String, message: &Message) -> YiZhanResult<()> {
        self.stream.writable().await?;
        let command_packet = encode_to_vec(&message, config::standard())?;
        self.stream.try_write(command_packet.as_slice())?;

        Ok(())
    }
}

// unsafe impl<C> Sync for YiZhanClient<C> {}
