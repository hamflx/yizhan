use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use async_trait::async_trait;
use bincode::{config, decode_from_slice, encode_to_vec};
use log::{info, warn};
use nanoid::nanoid;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use yizhan_protocol::command::{Command, CommandResponse};
use yizhan_protocol::message::{Message, WELCOME_MESSAGE};

use crate::error::YiZhanResult;
use crate::serve::Serve;

pub(crate) struct TcpServe {
    pub(crate) listener: TcpListener,
    pub(crate) buffer: Vec<u8>,
    pub(crate) cached_size: usize,
    pub(crate) client_map: Arc<Mutex<HashMap<String, Arc<TcpStream>>>>,
}

impl TcpServe {
    pub(crate) async fn new() -> YiZhanResult<Self> {
        Ok(Self {
            listener: TcpListener::bind("127.0.0.1:3777").await?,
            buffer: Vec::new(),
            cached_size: 0,
            client_map: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

#[async_trait]
impl Serve for TcpServe {
    async fn run(&self, sender: Sender<Message>) -> YiZhanResult<Message> {
        loop {
            let (stream, addr) = self.listener.accept().await?;
            let client_map = self.client_map.clone();
            let sender = sender.clone();
            info!("New client: {:?}", addr);
            spawn(async move { handle_client(stream, sender, client_map).await });
        }
    }

    async fn request(&self, cmd: Command) -> YiZhanResult<CommandResponse> {
        Ok(CommandResponse::Run(String::new()))
    }

    async fn get_peers(&self) -> YiZhanResult<Vec<String>> {
        let lock = self.client_map.lock().await;
        Ok(lock.keys().map(|k| k.clone()).collect())
    }

    async fn send(&self, client_id: String, message: &Message) -> YiZhanResult<()> {
        let lock = self.client_map.lock().await;
        if let Some(client) = lock.get(&client_id) {
            let packet = encode_to_vec(message, config::standard())?;
            client.try_write(packet.as_slice()).unwrap();
        } else {
            warn!("No client:{} found", client_id);
        }
        Ok(())
    }
}

async fn handle_client(
    stream: TcpStream,
    sender: Sender<Message>,
    client_map: Arc<Mutex<HashMap<String, Arc<TcpStream>>>>,
) -> YiZhanResult<()> {
    let conn_id = nanoid!();

    let stream = Arc::new(stream);
    handshake(stream.clone(), conn_id.as_str()).await?;

    let mut lock = client_map.lock().await;
    lock.insert(conn_id.clone(), stream.clone());
    drop(lock);

    let mut buffer = vec![0; 4096];
    let mut pos = 0;
    loop {
        let packet = read_packet(stream.clone(), &mut buffer, &mut pos).await?;
        if let Some(Message::Echo(client_id)) = packet.as_ref() {
            info!("Got echo packet");
            let mut lock = client_map.lock().await;
            lock.insert(client_id.to_string(), stream.clone());
        }
        if let Some(packet) = packet {
            info!("Got packet: {:?}", packet);
            sender.send(packet).await?;
        }
    }

    info!("End of handle_client");
}

async fn handshake(stream: Arc<TcpStream>, client_id: &str) -> YiZhanResult<()> {
    stream.writable().await?;

    let welcome_message = Message::Echo(client_id.to_string());
    stream.try_write(encode_to_vec(&welcome_message, config::standard())?.as_slice())?;

    Ok(())
}

async fn read_packet(
    stream: Arc<TcpStream>,
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
