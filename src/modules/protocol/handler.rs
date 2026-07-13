use crate::modules::protocol::message::Message;
use crate::modules::protocol::serializer::Serializer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

/// Responsible for reading from the stream
pub struct ReadHandler {
  reader: OwnedReadHalf,
  serializer: Box<dyn Serializer>,
}

impl ReadHandler {
  pub fn new(reader: OwnedReadHalf, serializer: Box<dyn Serializer>) -> Self {
    Self {
      reader,
      serializer,
    }
  }

  /// Reads a length-prefixed message from the stream.
  /// Returns None if the connection was cleanly closed.
  pub async fn receive(
    &mut self
  ) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
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

/// Responsible for writing to the stream
pub struct WriteHandler {
  writer: OwnedWriteHalf,
  serializer: Box<dyn Serializer>,
}

impl WriteHandler {
  pub fn new(writer: OwnedWriteHalf, serializer: Box<dyn Serializer>) -> Self {
    Self {
      writer,
      serializer,
    }
  }

  /// Writes a length-prefixed message to the stream.
  /// Format: [4 bytes: u32 big-endian length][N bytes: JSON body]
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
}

/// NOTE(slok): This is no longer needed
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

  /// Writes a length-prefixed message to the stream.
  /// Format: [4 bytes: u32 big-endian length][N bytes: JSON body]
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

  /// Reads a length-prefixed message from the stream.
  /// Returns None if the connection was cleanly closed.
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
