use crate::codec::{recv_message, send_message};
use crate::message::Message;
use tokio::net::{TcpListener, TcpStream};

pub async fn start(addr: &str) -> tokio::io::Result<()> {
  let listener = TcpListener::bind(addr).await?;
  println!("[listener] Bound to {}", addr);

  loop {
    let (socket, peer_addr) = listener.accept().await?;
    println!("[listener] Connection from {}", peer_addr);
    tokio::spawn(handle_connection(socket));
  }
}

async fn handle_connection(mut socket: TcpStream) {
  loop {
    match recv_message(&mut socket).await {
      Ok(Some(msg)) => {
        println!("[listener] Received: {:?}", msg);

        let reply = match msg {
          Message::Ping => Message::Pong,
          Message::Put { .. } => Message::Ok,
          Message::Get { key } => Message::Err {
            reason: format!("'{}' not implemented yet", key),
          },
          _ => Message::Err {
            reason: "unexpected message".into(),
          },
        };

        if let Err(e) = send_message(&mut socket, &reply).await {
          eprintln!("[listener] Write error: {}", e);
          return;
        }
      }
      Ok(None) => {
        println!("[listener] Connection closed");
        return;
      }
      Err(e) => {
        eprintln!("[listener] Error: {}", e);
        return;
      }
    }
  }
}
