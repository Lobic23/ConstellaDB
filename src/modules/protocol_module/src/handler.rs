use crate::message::Message;
use crate::serializer::Serializer;
use std::io::{Read, Write};
use std::net::TcpStream;

pub struct ProtocolHandler {
    stream: TcpStream,
    serializer: Box<dyn Serializer>,
}

impl ProtocolHandler {
    pub fn new(stream: TcpStream, serializer: Box<dyn Serializer>) -> Self {
        ProtocolHandler { stream, serializer }
    }

    pub fn send(&mut self, message: &Message) -> Result<(), Box<dyn std::error::Error>> {
        let data = self.serializer.serialize(message)?;
        let len = data.len() as u32;

        self.stream.write_all(&len.to_be_bytes())?;
        self.stream.write_all(&data)?;
        self.stream.flush()?;

        Ok(())
    }

    pub fn receive(&mut self) -> Result<Message, Box<dyn std::error::Error>> {
        let mut len_bytes = [0u8; 4];
        self.stream.read_exact(&mut len_bytes)?;

        let len = u32::from_be_bytes(len_bytes) as usize;
        let mut buffer = vec![0u8; len];
        self.stream.read_exact(&mut buffer)?;

        self.serializer.deserialize(&buffer)
            .map_err(|e| e.into())
    }
}