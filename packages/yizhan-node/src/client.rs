use std::{io, sync::Arc};

use async_trait::async_trait;
use bincode::{config, de::read::SliceReader, decode_from_slice};
use log::info;
use serde::{Deserialize, Deserializer};
use tokio::{
    io::{stdin, AsyncReadExt},
    net::TcpStream,
    select, spawn,
    sync::mpsc::channel,
};
use yizhan_protocol::message::Message;

use crate::{connection::Connection, console::Console, error::YiZhanResult};

pub(crate) struct YiZhanClient<C> {
    console: Arc<Box<C>>,
}

impl<C: Console> YiZhanClient<C> {
    pub fn new(console: C) -> Self {
        Self {
            console: Arc::new(Box::new(console)),
        }
    }

    async fn handle_remote_message(
        &self,
        stream: &TcpStream,
        buffer: &mut [u8],
        cached_size: &mut usize,
    ) -> YiZhanResult<bool> {
        let remains_buffer = &mut buffer[*cached_size..];
        if remains_buffer.is_empty() {
            return Err(anyhow::anyhow!("No enough space"));
        }

        let size = match stream.try_read(remains_buffer) {
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(true),
            Err(err) => return Err(err.into()),
            Ok(0) => return Ok(false),
            Ok(size) => size,
        };

        *cached_size += size;
        let packet = &buffer[..*cached_size];

        let message: Message = match decode_from_slice(packet, config::standard()) {
            Ok((msg, len)) => {
                *cached_size -= len;
                msg
            }
            Err(_) => return Ok(true),
        };

        println!("Got message: {:?}", message);

        Ok(true)
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
impl<C: Console + Send + Sync + 'static> Connection for YiZhanClient<C> {
    async fn run(&self) -> YiZhanResult<()> {
        let stream = TcpStream::connect("127.0.0.1:3777").await?;

        let (cmd_tx, mut cmd_rx) = channel(40960);
        let console = self.console.clone();
        spawn(async move { console.run(cmd_tx).await });

        let mut buffer = vec![0; 40960];
        let mut cached_size = 0;

        loop {
            select! {
                cmd_res = cmd_rx.recv() => {
                    if let Some(cmd) = cmd_res {
                        info!("Got command {:?}", cmd);
                        stream.writable().await?;
                        // stream.try_write(buf)
                    }
                }
                _ = stream.readable() => {
                    self.handle_remote_message(&stream,  buffer.as_mut_slice(), &mut cached_size).await?;
                }
            }
        }
    }
}

unsafe impl<C> Sync for YiZhanClient<C> {}
