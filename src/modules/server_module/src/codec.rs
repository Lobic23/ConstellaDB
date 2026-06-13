use protocol_module::message::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

pub async fn read_message(reader: &mut OwnedReadHalf) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
    let mut len_bytes = [0u8; 4];
    reader.read_exact(&mut len_bytes).await?;

    let len = u32::from_be_bytes(len_bytes) as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;

    let msg = bincode::deserialize(&buf)?;
    Ok(msg)
}

pub async fn write_message(writer: &mut OwnedWriteHalf, msg: &Message) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data = bincode::serialize(msg)?;
    let len = (data.len() as u32).to_be_bytes();

    writer.write_all(&len).await?;
    writer.write_all(&data).await?;
    writer.flush().await?;

    Ok(())
}