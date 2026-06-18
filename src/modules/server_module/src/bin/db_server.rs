use cmd_module::{execute, parse_cmd};
use db_module::Engine;
use protocol_module::handler::ProtocolHandler;
use protocol_module::message::{Message, MessageType};
use protocol_module::serializer::BincodeSerializer;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
  let addr = "0.0.0.0:7878";
  let engine = Arc::new(Mutex::new(Engine::new()));

  let listener = TcpListener::bind(addr).await.unwrap();
  println!("[db_server] listening on {}", addr);

  loop {
    let (stream, peer) = listener.accept().await.unwrap();
    println!("[db_server] client connected: {}", peer);

    let engine = Arc::clone(&engine);

    tokio::spawn(async move {
      let mut handler = ProtocolHandler::new(stream, Box::new(BincodeSerializer));

      loop {
        match handler.receive().await {
          Ok(msg) => {
            if msg.msg_type != MessageType::Query {
              eprintln!("[db_server] unexpected msg type from {}", peer);
              continue;
            }

            let sql = match String::from_utf8(msg.payload.clone()) {
              Ok(s) => s,
              Err(_) => {
                let err = Message::new(msg.id, MessageType::Error, "server".to_string())
                  .with_payload(b"invalid UTF-8 payload".to_vec());
                let _ = handler.send(&err).await;
                continue;
              }
            };

            println!("[db_server] {} >>> {}", peer, sql.trim());

            let result = match parse_cmd(&sql) {
              Ok(cmd) => {
                let mut eng = engine.lock().await;
                execute(&mut eng, cmd)
              }
              Err(e) => format!("Parse error: {e}"),
            };

            println!("[db_server] {}", result);

            let response = Message::new(msg.id, MessageType::Response, "server".to_string())
              .with_payload(result.into_bytes());

            if let Err(e) = handler.send(&response).await {
              eprintln!("[db_server] write error for {}: {}", peer, e);
              break;
            }
          }
          Err(e) => {
            println!("[db_server] client {} disconnected: {}", peer, e);
            break;
          }
        }
      }
    });
  }
}
