use std::{io, sync::Arc};

use async_trait::async_trait;
use bincode::{config, de::read::SliceReader, decode_from_slice, encode_to_vec};
use log::info;
use nanoid::nanoid;
use serde::{Deserialize, Deserializer};
use tokio::{
    io::{stdin, AsyncReadExt},
    net::TcpStream,
    select, spawn,
    sync::mpsc::{channel, Sender},
};
use yizhan_protocol::{
    command::{Command, CommandResponse},
    message::Message,
};

use crate::{connection::Connection, console::Console, error::YiZhanResult};

pub(crate) struct YiZhanClient {
    stream: TcpStream,
}

impl YiZhanClient {
    pub(crate) async fn new() -> YiZhanResult<Self> {
        Ok(Self {
            stream: TcpStream::connect("127.0.0.1:3777").await?,
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

#[async_trait]
impl Connection for YiZhanClient {
    async fn run(&self, sender: Sender<Message>) -> YiZhanResult<Message> {
        let client_id = nanoid!();
        let (cmd_tx, mut cmd_rx) = channel(40960);

        let mut buffer = vec![0; 40960];
        let mut cached_size = 0;

        loop {
            select! {
                cmd_res = cmd_rx.recv() => {
                    if let Some(cmd) = cmd_res {
                        info!("Got command {:?}", cmd);
                        self.stream.writable().await?;
                        let command_packet = encode_to_vec(
                            &Message::Command(nanoid!(), cmd),
                            config::standard(),
                        )?;
                        self.stream.try_write(command_packet.as_slice())?;
                    }
                }
                _ = self.stream.readable() => {
                    if let Some(msg) = self.handle_remote_message(&self.stream,  buffer.as_mut_slice(), &mut cached_size).await? {
                        match &msg {
                            Message::Echo(server_id) => {
                                info!("Sending echo");
                                self.stream.writable().await?;
                                let echo_packet = encode_to_vec(
                                    &Message::Echo(client_id.to_string()),
                                    config::standard(),
                                )?;
                                self.stream.try_write(echo_packet.as_slice())?;
                            },
                            _ => {}
                        }
                        sender.send(msg).await?;
                    }
                }
            }
        }
    }

    async fn request(&self, cmd: Command) -> YiZhanResult<CommandResponse> {
        Ok(CommandResponse::Run(String::new()))
    }

    async fn send(&self, client_id: String, message: &Message) -> YiZhanResult<()> {
        self.stream.writable().await?;
        let command_packet = encode_to_vec(&message, config::standard())?;
        self.stream.try_write(command_packet.as_slice())?;

        Ok(())
    }
}

// unsafe impl<C> Sync for YiZhanClient<C> {}
