use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use crate::message::Message;

/// Writes a length-prefixed message to the stream.
/// Format: [4 bytes: u32 big-endian length][N bytes: JSON body]
pub async fn send_message(stream: &mut TcpStream, msg: &Message) -> tokio::io::Result<()> {
    let body = serde_json::to_vec(msg)
        .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, e))?;

    let len = body.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(&body).await?;
    Ok(())
}

/// Reads a length-prefixed message from the stream.
/// Returns None if the connection was cleanly closed.
pub async fn recv_message(stream: &mut TcpStream) -> tokio::io::Result<Option<Message>> {
    let mut len_buf = [0u8; 4];

    // read_exact returns UnexpectedEof if connection closed mid-read
    match stream.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == tokio::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }

    let len = u32::from_be_bytes(len_buf) as usize;

    let mut body = vec![0u8; len];
    stream.read_exact(&mut body).await?;

    let msg = serde_json::from_slice(&body)
        .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::InvalidData, e))?;

    Ok(Some(msg))
}