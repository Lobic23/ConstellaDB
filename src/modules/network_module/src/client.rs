use std::collections::HashMap;
use tokio::net::TcpStream;
use crate::message::Message;
use crate::codec::{send_message, recv_message};

pub struct ConnectionPool {
    conns: HashMap<String, TcpStream>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            conns: HashMap::new(),
        }
    }

    /// Returns existing connection or dials a new one
    pub async fn get_or_connect(&mut self, addr: &str) -> tokio::io::Result<&mut TcpStream> {
        if !self.conns.contains_key(addr) {
            let stream = TcpStream::connect(addr).await?;
            println!("[pool] New connection to {}", addr);
            self.conns.insert(addr.to_string(), stream);
        } else {
            println!("[pool] Reusing connection to {}", addr);
        }

        Ok(self.conns.get_mut(addr).unwrap())
    }

    /// Send a message to a peer, reusing connection if possible
    pub async fn send(&mut self, addr: &str, msg: &Message) -> tokio::io::Result<Option<Message>> {
        let stream = self.get_or_connect(addr).await?;
        send_message(stream, msg).await?;
        recv_message(stream).await
    }

    /// Drop a connection (e.g. on error)
    pub fn remove(&mut self, addr: &str) {
        self.conns.remove(addr);
        println!("[pool] Dropped connection to {}", addr);
    }
}