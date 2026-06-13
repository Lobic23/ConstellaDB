use protocol_module::message::{Message, MessageType};
use protocol_module::handler::ProtocolHandler;
use protocol_module::serializer::BincodeSerializer;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;

pub struct Client {
    node_id: String,
    handler: ProtocolHandler,
    next_id: u64,
}

impl Client {
    pub async fn connect(addr: &str, node_id: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let stream = TcpStream::connect(addr).await?;
        println!("[client] connected to {}", addr);

        Ok(Client {
            node_id: node_id.to_string(),
            handler: ProtocolHandler::new(stream, Box::new(BincodeSerializer)),
            next_id: 0,
        })
    }

    pub async fn ping(&mut self) -> Result<Duration, Box<dyn std::error::Error + Send + Sync>> {
        let id = self.next_id;
        self.next_id += 1;

        let ping = Message::new(id, MessageType::Heartbeat, self.node_id.clone())
            .with_payload(b"PING".to_vec());

        let start = Instant::now();
        self.handler.send(&ping).await?;

        let response = self.handler.receive().await?;
        let rtt = start.elapsed();

        if response.msg_type == MessageType::Heartbeat && response.payload == b"PONG" {
            println!("[client] PONG received (id={}, rtt={:?})", id, rtt);
            Ok(rtt)
        } else {
            Err("unexpected response to PING".into())
        }
    }
}