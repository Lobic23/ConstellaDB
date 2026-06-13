use protocol_module::message::{Message, MessageType};
use protocol_module::handler::ProtocolHandler;
use protocol_module::serializer::BincodeSerializer;
use tokio::net::TcpListener;

pub struct Server {
    addr: String,
}

impl Server {
    pub fn new(addr: &str) -> Self {
        Server { addr: addr.to_string() }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("[server] listening on {}", self.addr);

        loop {
            let (stream, peer) = listener.accept().await?;
            println!("[server] client connected: {}", peer);

            tokio::spawn(async move {
                let mut handler = ProtocolHandler::new(stream, Box::new(BincodeSerializer));

                loop {
                    match handler.receive().await {
                        Ok(msg) => {
                            if msg.msg_type == MessageType::Heartbeat
                                && msg.payload == b"PING"
                            {
                                println!("[server] PING from {} (node: {})", peer, msg.node_id);

                                let pong = Message::new(msg.id, MessageType::Heartbeat, "server".to_string())
                                    .with_payload(b"PONG".to_vec());

                                if let Err(e) = handler.send(&pong).await {
                                    eprintln!("[server] write error for {}: {}", peer, e);
                                    break;
                                }
                            } else {
                                eprintln!("[server] unexpected message type from {}", peer);
                            }
                        }
                        Err(e) => {
                            println!("[server] client {} disconnected: {}", peer, e);
                            break;
                        }
                    }
                }
            });
        }
    }
}