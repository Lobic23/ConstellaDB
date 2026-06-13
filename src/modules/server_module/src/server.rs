use crate::codec::{read_message, write_message};
use protocol_module::message::{Message, MessageType};
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
                let (mut reader, mut writer) = stream.into_split();

                loop {
                    match read_message(&mut reader).await {
                        Ok(msg) => {
                            if msg.msg_type == MessageType::Heartbeat
                                && msg.payload == b"PING"
                            {
                                println!("[server] PING from {} (node: {})", peer, msg.node_id);

                                let pong = Message::new(msg.id, MessageType::Heartbeat, "server".to_string())
                                    .with_payload(b"PONG".to_vec());

                                if let Err(e) = write_message(&mut writer, &pong).await {
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