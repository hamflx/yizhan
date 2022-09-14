use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio::{select, spawn};
use tracing::{debug, info, warn};
use yizhan_common::error::YiZhanResult;
use yizhan_protocol::command::{ListedNodeInfo, NodeInfo};
use yizhan_protocol::message::Message;

use crate::config::YiZhanServerConfig;
use crate::context::YiZhanContext;
use crate::message::{read_packet, send_packet, ReadPacketResult};
use crate::serve::Serve;

pub(crate) type ClientMap = Mutex<HashMap<String, (ListedNodeInfo, Arc<TcpStream>)>>;

pub(crate) struct TcpServe {
    pub(crate) listener: TcpListener,
    pub(crate) client_map: Arc<ClientMap>,
    pub(crate) sub_tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl TcpServe {
    pub(crate) async fn new(config: &YiZhanServerConfig) -> YiZhanResult<Self> {
        Ok(Self {
            listener: TcpListener::bind(config.listen.as_str()).await?,
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
            let ctx = ctx.clone();
            info!("New client: {:?}", addr);
            let task = spawn({
                let shut_rx = shut_rx.resubscribe();
                async move {
                    let mut peer_node_id = None;
                    if let Err(err) = handle_client(
                        addr,
                        shut_rx,
                        ctx,
                        &mut peer_node_id,
                        stream,
                        sender,
                        &client_map,
                    )
                    .await
                    {
                        warn!("An error occurred when handle_client: {:?}", err);
                    } else {
                        info!("handle_client end");
                    }
                    if let Some(node_id) = peer_node_id {
                        let mut lock = client_map.lock().await;
                        lock.remove(&node_id);
                    }
                }
            });
            let mut sub_tasks = self.sub_tasks.lock().await;
            sub_tasks.push(task);
        }

        Ok(())
    }

    async fn get_peers(&self) -> YiZhanResult<Vec<ListedNodeInfo>> {
        let lock = self.client_map.lock().await;
        Ok(lock.values().map(|v| v.0.clone()).collect())
    }

    async fn send(&self, node_id: String, message: &Message) -> YiZhanResult<()> {
        info!("send trying lock client_map");
        let mut lock = self.client_map.lock().await;
        info!("send got locked client_map");
        if let Some(client) = lock.get(&node_id) {
            info!("Sending packet to {}", node_id);
            if let Err(err) = send_packet(&client.1, message).await {
                warn!("send_packet error: {:?}", err);
                lock.remove(&node_id);
                return Err(err);
            }
            info!("Sent packet to {}", node_id);
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
    addr: SocketAddr,
    mut shut_rx: Receiver<()>,
    ctx: Arc<YiZhanContext>,
    peer_node_id: &mut Option<String>,
    stream: TcpStream,
    sender: Sender<(String, Message)>,
    client_map: &ClientMap,
) -> YiZhanResult<()> {
    let stream = Arc::new(stream);
    handshake(&stream, &ctx).await?;

    let mut buffer = vec![0; 10485760 * 2];
    let mut pos = 0;
    let mut last_msg_time = SystemTime::now();
    loop {
        let readable = select! {
            _ = shut_rx.recv() => break,
            r = stream.readable() => Some(r?),
            _ = sleep(Duration::from_secs(15)) => None
        };

        if readable.is_none() {
            let duration = last_msg_time.elapsed()?.as_secs();
            if duration > 60 {
                return Err(anyhow::anyhow!(
                    "no message received in {} seconds",
                    duration
                ));
            }
            send_packet(&stream, &Message::Heartbeat).await?;
            continue;
        }

        info!("Some data readable");
        let packet = read_packet(&stream, &mut buffer, &mut pos).await?;
        last_msg_time = SystemTime::now();

        match packet {
            ReadPacketResult::None => break,
            ReadPacketResult::Some(Message::Heartbeat) => {
                debug!("Received heartbeat");
            }
            ReadPacketResult::Some(packet) if packet != Message::Heartbeat => {
                info!("Received packet");
                if let Message::Echo(client_info) = &packet {
                    *peer_node_id = Some(client_info.id.to_string());
                    info!("Client {} connected", client_info.id);
                    let mut lock = client_map.lock().await;
                    lock.insert(
                        client_info.id.to_string(),
                        (
                            ListedNodeInfo {
                                id: client_info.id.clone(),
                                mac: client_info.mac.clone(),
                                version: client_info.version.clone(),
                                ip: addr.to_string(),
                            },
                            stream.clone(),
                        ),
                    );
                }
                if let Some(peer_node_id) = &peer_node_id {
                    sender.send((peer_node_id.to_string(), packet)).await?;
                } else {
                    warn!("No peer client_id");
                }
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handshake(stream: &TcpStream, ctx: &YiZhanContext) -> YiZhanResult<()> {
    stream.writable().await?;

    let node_info = NodeInfo {
        id: ctx.name.to_string(),
        // todo mac 地址。
        mac: String::new(),
        version: ctx.version.clone(),
    };
    let welcome_message = Message::Echo(node_info);
    send_packet(stream, &welcome_message).await?;

    Ok(())
}
