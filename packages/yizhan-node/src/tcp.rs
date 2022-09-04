use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use bincode::{config, encode_to_vec};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::{select, spawn};
use tracing::{info, warn};
use yizhan_protocol::message::Message;

use crate::context::YiZhanContext;
use crate::error::YiZhanResult;
use crate::message::{read_packet, send_packet, ReadPacketResult};
use crate::serve::Serve;

pub(crate) struct TcpServe {
    pub(crate) listener: TcpListener,
    pub(crate) client_map: Arc<Mutex<HashMap<String, Arc<TcpStream>>>>,
    pub(crate) sub_tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl TcpServe {
    pub(crate) async fn new() -> YiZhanResult<Self> {
        Ok(Self {
            // todo 改为 127.0.0.1 并提供配置方式。
            listener: TcpListener::bind("0.0.0.0:3777").await?,
            client_map: Arc::new(Mutex::new(HashMap::new())),
            sub_tasks: Mutex::new(Vec::new()),
        })
    }
}

#[async_trait]
impl Serve for TcpServe {
    async fn run(
        &self,
        ctx: Arc<YiZhanContext>,
        sender: Sender<(String, Message)>,
        mut shut_rx: Receiver<()>,
    ) -> YiZhanResult<()> {
        loop {
            info!("Waiting for client connection");

            let (stream, addr) = select! {
                _ = shut_rx.recv() => break,
                c = self.listener.accept() => c?,
            };
            let client_map = self.client_map.clone();
            let sender = sender.clone();
            let name = ctx.name.to_string();
            info!("New client: {:?}", addr);
            let task = spawn({
                let shut_rx = shut_rx.resubscribe();
                async move {
                    if let Err(err) =
                        handle_client(shut_rx, name.as_str(), stream, sender, client_map).await
                    {
                        warn!("An error occurred when handle_client: {:?}", err);
                    } else {
                        info!("handle_client end");
                    }
                }
            });
            let mut sub_tasks = self.sub_tasks.lock().await;
            sub_tasks.push(task);
        }

        Ok(())
    }

    async fn get_peers(&self) -> YiZhanResult<Vec<String>> {
        let lock = self.client_map.lock().await;
        Ok(lock.keys().cloned().collect())
    }

    async fn send(&self, node_id: String, message: &Message) -> YiZhanResult<()> {
        let lock = self.client_map.lock().await;
        if let Some(client) = lock.get(&node_id) {
            info!("Sending packet to {}", node_id);
            send_packet(client, message).await?;
        } else {
            warn!("No client:{} found", node_id);
        }
        Ok(())
    }

    async fn flush(&self) -> YiZhanResult<()> {
        // todo flush ???????
        // let mut client_map = self.client_map.lock().await;
        // for stream in client_map.values_mut() {
        //     let _ = stream.flush().await;
        // }
        Ok(())
    }
}

async fn handle_client(
    mut shut_rx: Receiver<()>,
    name: &str,
    stream: TcpStream,
    sender: Sender<(String, Message)>,
    client_map: Arc<Mutex<HashMap<String, Arc<TcpStream>>>>,
) -> YiZhanResult<()> {
    let stream = Arc::new(stream);
    handshake(&stream, name).await?;

    let mut buffer = vec![0; 10485760];
    let mut pos = 0;
    let mut peer_client_id = None;
    loop {
        select! {
            _ = shut_rx.recv() => break,
            r = stream.readable() => {
                r?;
            }
        };

        let packet = read_packet(&stream, &mut buffer, &mut pos).await?;
        match packet {
            ReadPacketResult::None => break,
            ReadPacketResult::Some(packet) => {
                if let Message::Echo(client_id) = &packet {
                    peer_client_id = Some(client_id.to_string());
                    info!("Got echo packet");
                    let mut lock = client_map.lock().await;
                    lock.insert(client_id.to_string(), stream.clone());
                }
                if let Some(peer_client_id) = &peer_client_id {
                    sender.send((peer_client_id.to_string(), packet)).await?;
                } else {
                    warn!("No peer client_id");
                }
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handshake(stream: &TcpStream, client_id: &str) -> YiZhanResult<()> {
    stream.writable().await?;

    let welcome_message = Message::Echo(client_id.to_string());
    stream.try_write(encode_to_vec(&welcome_message, config::standard())?.as_slice())?;

    Ok(())
}
