use std::io;

use async_trait::async_trait;
use bincode::{config, decode_from_slice, encode_to_vec};
use log::info;
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
            let (stream, addr) = listner.accept().await?;
            info!("New client: {:?}", addr);
            spawn(handle_client(stream));
        }
    }
}

async fn handle_client(stream: TcpStream) -> YiZhanResult<()> {
    handshake(&stream).await?;

    let mut buffer = vec![0; 4096];
    let mut pos = 0;
    loop {
        let packet = read_packet(&stream, &mut buffer, &mut pos).await?;
        info!("Got packet: {:?}", packet);
    }
}

async fn handshake(stream: &TcpStream) -> YiZhanResult<()> {
    stream.writable().await?;

    let welcome_message = Message::Echo(WELCOME_MESSAGE.to_string());
    stream.try_write(encode_to_vec(&welcome_message, config::standard())?.as_slice())?;

    Ok(())
}

async fn read_packet(
    stream: &TcpStream,
    buffer: &mut Vec<u8>,
    pos: &mut usize,
) -> YiZhanResult<Option<Message>> {
    loop {
        stream.readable().await?;

        let remains_buffer = &mut buffer[*pos..];
        if remains_buffer.is_empty() {
            return Err(anyhow::anyhow!("No enough space"));
        }
        let bytes_read = match stream.try_read(remains_buffer) {
            Ok(0) => return Err(anyhow::anyhow!("No enough data")),
            Ok(n) => n,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(err) => return Err(err.into()),
        };
        *pos += bytes_read;

        if let Ok((msg, size)) = decode_from_slice(&buffer.as_slice()[..*pos], config::standard()) {
            buffer.drain(..size);
            *pos -= size;
            return Ok(Some(msg));
        }
    }
}
