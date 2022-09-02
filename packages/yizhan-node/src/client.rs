use std::sync::Arc;

use async_trait::async_trait;
use bincode::{config, encode_to_vec};
use tokio::{
    net::TcpStream,
    sync::{broadcast::Receiver, mpsc::Sender, Mutex},
};
use tracing::{info, warn};
use yizhan_protocol::message::Message;

use crate::{
    connection::Connection,
    context::YiZhanContext,
    error::YiZhanResult,
    message::{read_packet, send_packet},
};

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
}

#[async_trait]
impl Connection for YiZhanClient {
    async fn run(
        &self,
        ctx: Arc<YiZhanContext>,
        sender: Sender<(String, Message)>,
        mut shut_rx: Receiver<()>,
    ) -> YiZhanResult<()> {
        let mut buffer = vec![0; 10485760];
        let mut pos = 0;

        let mut peer_node_id = None;

        loop {
            let msg = read_packet(&self.stream, &mut shut_rx, &mut buffer, &mut pos).await?;
            match msg {
                None => break,
                Some(msg) => {
                    if let Message::Echo(server_id) = &msg {
                        peer_node_id = Some(server_id.to_string());
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
                    if let Some(peer_node_id) = &peer_node_id {
                        info!("Got some packet");
                        sender.send((peer_node_id.to_string(), msg)).await?;
                    } else {
                        warn!("No peer id");
                    }
                }
            }
        }

        Ok(())
    }

    async fn get_peers(&self) -> YiZhanResult<Vec<String>> {
        let lock = self.peer_id.lock().await;
        Ok(lock.as_ref().map(|id| vec![id.clone()]).unwrap_or_default())
    }

    async fn send(&self, _client_id: String, message: &Message) -> YiZhanResult<()> {
        send_packet(&self.stream, message).await
    }

    async fn flush(&self) -> YiZhanResult<()> {
        // todo flush
        Ok(())
    }
}
