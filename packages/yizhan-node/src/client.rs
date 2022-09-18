use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use async_trait::async_trait;
use bincode::{config, encode_to_vec};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    select,
    sync::{
        broadcast::Receiver,
        mpsc::{self, Sender},
        Mutex,
    },
    time::sleep,
};
use tracing::{debug, info, warn};
use yizhan_common::error::YiZhanResult;
use yizhan_protocol::{
    command::{ListedNodeInfo, NodeInfo},
    message::Message,
};

use crate::{
    connection::Connection,
    context::YiZhanContext,
    message::{read_packet, ReadPacketResult},
};

pub(crate) struct YiZhanClient {
    peer_id: Mutex<Option<ListedNodeInfo>>,
    tx_channel: mpsc::Sender<Message>,
    rx_channel: Mutex<mpsc::Receiver<Message>>,
}

impl YiZhanClient {
    pub(crate) fn new() -> YiZhanResult<Self> {
        let (tx, rx) = mpsc::channel(50);
        Ok(Self {
            peer_id: Mutex::new(None),
            tx_channel: tx,
            rx_channel: Mutex::new(rx),
        })
    }

    async fn run_message_loop(
        &self,
        ctx: &YiZhanContext,
        stream: &mut TcpStream,
        sender: &Sender<(String, Message)>,
        shut_rx: &mut Receiver<()>,
    ) -> YiZhanResult<()> {
        let mut rx = self.rx_channel.lock().await;

        let mut buffer = vec![0; 10485760];
        let mut pos = 0;

        let mut peer_node_id = None;
        let mut last_msg_time = SystemTime::now();

        loop {
            info!("Waiting for event ...");
            let (readable, recv, timedout, out_packet) = select! {
                _ = shut_rx.recv() => break,
                r = stream.readable() => {
                    r?;
                    (true, false, false, None)
                },
                packet = rx.recv() => (false, true, false, packet),
                _ = sleep(Duration::from_secs(15)) => (false, false, true, None)
            };

            if timedout {
                let duration = last_msg_time.elapsed()?.as_secs();
                if duration > 60 {
                    return Err(anyhow::anyhow!(
                        "no message received in {} seconds",
                        duration
                    ));
                }

                stream
                    .write_all(&encode_to_vec(&Message::Heartbeat, config::standard())?)
                    .await?;
                stream.flush().await?;
                continue;
            }

            if readable {
                info!("Some data arrived");
                last_msg_time = SystemTime::now();
                let msg = read_packet(stream, &mut buffer, &mut pos).await?;
                match msg {
                    ReadPacketResult::None => break,
                    ReadPacketResult::Some(Message::Heartbeat) => {
                        debug!("Received heartbeat");
                    }
                    ReadPacketResult::Some(msg) => {
                        if let Message::Echo(server_info) = &msg {
                            peer_node_id = Some(server_info.id.to_string());
                            info!("Got echo packet: {:?}", server_info);

                            let mut lock = self.peer_id.lock().await;
                            *lock = Some(ListedNodeInfo {
                                id: server_info.id.clone(),
                                mac: Vec::new(),
                                version: server_info.version.clone(),
                                ip: stream.peer_addr().unwrap().to_string(),
                            });
                            drop(lock);

                            stream.writable().await?;
                            let self_info = NodeInfo {
                                id: ctx.name.to_string(),
                                // todo 加入 mac 地址。
                                mac: Vec::new(),
                                version: ctx.version.clone(),
                            };
                            let echo_packet =
                                encode_to_vec(&Message::Echo(self_info), config::standard())?;
                            stream.write_all(&echo_packet).await?;
                            stream.flush().await?;
                        }
                        if let Some(peer_node_id) = &peer_node_id {
                            info!("Got some packet");
                            sender.send((peer_node_id.to_string(), msg)).await?;
                        } else {
                            warn!("No peer id");
                        }
                    }
                    _ => {}
                }
            }

            if recv {
                if let Some(out_packet) = out_packet {
                    info!("Sending packet ...");
                    stream.writable().await?;
                    let command_packet = encode_to_vec(&out_packet, config::standard())?;
                    stream.write_all(command_packet.as_slice()).await?;
                    stream.flush().await?;
                    info!("Packet sent to server");
                } else {
                    info!("Receiver closed");
                    break;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Connection for YiZhanClient {
    async fn run(
        &self,
        ctx: Arc<YiZhanContext>,
        sender: Sender<(String, Message)>,
        mut shut_rx: Receiver<()>,
    ) -> YiZhanResult<()> {
        let server_addr = format!("{}:{}", ctx.config.client.host, ctx.config.client.port);
        loop {
            info!("Connecting to server ...");
            let stream = select! {
                r = TcpStream::connect(&server_addr) => r,
                _ = shut_rx.recv() => break
            };

            match stream {
                Ok(mut stream) => {
                    info!("Connected to server");
                    if let Err(err) = self
                        .run_message_loop(&ctx, &mut stream, &sender, &mut shut_rx)
                        .await
                    {
                        info!("Error: {:?}", err);
                    }
                }
                Err(err) => warn!("Failed to connect to server: {:?}", err),
            }

            sleep(Duration::from_secs(15)).await;
        }

        Ok(())
    }

    async fn get_peers(&self) -> YiZhanResult<Vec<ListedNodeInfo>> {
        let lock = self.peer_id.lock().await;
        Ok(lock
            .as_ref()
            .map(|info| vec![info.clone()])
            .unwrap_or_default())
    }

    async fn send(&self, _client_id: String, message: Message) -> YiZhanResult<()> {
        self.tx_channel.send(message).await?;
        Ok(())
    }

    async fn flush(&self) -> YiZhanResult<()> {
        // todo flush
        Ok(())
    }
}
