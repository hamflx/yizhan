use std::io;

use bincode::{config, decode_from_slice, encode_to_vec};
use tokio::net::TcpStream;
use tracing::info;
use yizhan_common::error::YiZhanResult;
use yizhan_protocol::message::Message;

pub(crate) enum ReadPacketResult {
    Some(Message),
    None,
    WouldBlock,
}

pub(crate) async fn read_packet(
    stream: &TcpStream,
    buffer: &mut Vec<u8>,
    pos: &mut usize,
) -> YiZhanResult<ReadPacketResult> {
    let mut eof = false;
    while !eof {
        let remains_buffer = &mut buffer[*pos..];
        if remains_buffer.is_empty() {
            return Err(anyhow::anyhow!("No enough space"));
        }
        let bytes_read = match stream.try_read(remains_buffer) {
            Ok(0) => {
                eof = true;
                info!("End of stream");
                0
            }
            Ok(n) => n,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                return Ok(ReadPacketResult::WouldBlock)
            }
            Err(err) => return Err(err.into()),
        };
        *pos += bytes_read;

        if let Ok((msg, size)) = decode_from_slice(&buffer.as_slice()[..*pos], config::standard()) {
            info!("Got packet");
            buffer.copy_within(size..*pos, 0);
            *pos -= size;
            return Ok(ReadPacketResult::Some(msg));
        }
    }

    if *pos > 0 {
        Err(anyhow::anyhow!("No enough data"))
    } else {
        Ok(ReadPacketResult::None)
    }
}

pub(crate) async fn send_packet(stream: &TcpStream, message: &Message) -> YiZhanResult<()> {
    let command_packet = encode_to_vec(message, config::standard())?;
    let total_size = command_packet.len();
    let command_bytes = command_packet.as_slice();
    let mut bytes_sent = 0;
    while bytes_sent != total_size {
        stream.writable().await?;
        let n = match stream.try_write(&command_bytes[bytes_sent..]) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Ok(n) => n,
            Err(err) => return Err(err.into()),
        };
        bytes_sent += n;
    }

    Ok(())
}
