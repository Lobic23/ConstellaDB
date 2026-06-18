use crate::message::Message;
use crate::serializer::Serializer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

pub struct ProtocolHandler {
  reader: OwnedReadHalf,
  writer: OwnedWriteHalf,
  serializer: Box<dyn Serializer>,
}

impl ProtocolHandler {
  pub fn new(stream: tokio::net::TcpStream, serializer: Box<dyn Serializer>) -> Self {
    let (reader, writer) = stream.into_split();
    ProtocolHandler {
      reader,
      writer,
      serializer,
    }
  }

  pub async fn send(
    &mut self,
    message: &Message,
  ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data = self
      .serializer
      .serialize(message)
      .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;

    let len = (data.len() as u32).to_be_bytes();
    self.writer.write_all(&len).await?;
    self.writer.write_all(&data).await?;
    self.writer.flush().await?;

    Ok(())
  }

  pub async fn receive(&mut self) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
    let mut len_bytes = [0u8; 4];
    self.reader.read_exact(&mut len_bytes).await?;

    let len = u32::from_be_bytes(len_bytes) as usize;
    let mut buffer = vec![0u8; len];
    self.reader.read_exact(&mut buffer).await?;

    self
      .serializer
      .deserialize(&buffer)
      .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))
  }
}
