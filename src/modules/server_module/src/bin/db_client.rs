use uuid::Uuid;
use protocol_module::handler::ProtocolHandler;
use protocol_module::message::{Message, MessageType};
use protocol_module::serializer::BincodeSerializer;
use std::io::{self, Write};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
  let addr = std::env::args()
    .nth(1)
    .unwrap_or_else(|| "127.0.0.1:7979".to_string());

  let node_id = std::env::args()
    .nth(2)
    .unwrap_or_else(|| "client".to_string());

  let stream = TcpStream::connect(&addr).await.unwrap();
  println!("[db_client] connected to {} as '{}'", addr, node_id);
  println!("[db_client] type SQL and press Enter. Ctrl+C to quit.\n");

  let mut handler = ProtocolHandler::new(stream, Box::new(BincodeSerializer));
  let stdin = io::stdin();

  loop {
    print!("constelladb> ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    match stdin.read_line(&mut input) {
      Ok(0) => break,
      Ok(_) => {}
      Err(e) => {
        eprintln!("stdin error: {e}");
        break;
      }
    }

    let sql = input.trim();
    if sql.is_empty() {
      continue;
    }

    let msg_id = Uuid::new_v4().to_string();
    let msg = Message::new(msg_id, MessageType::Query, node_id.clone())
      .with_payload(sql.as_bytes().to_vec());

    if let Err(e) = handler.send(&msg).await {
      eprintln!("send error: {e}");
      break;
    }

    match handler.receive().await {
      Ok(resp) => println!("{}", String::from_utf8_lossy(&resp.payload)),
      Err(e) => {
        eprintln!("recv error: {e}");
        break;
      }
    }
  }
}
