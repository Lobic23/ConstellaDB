use std::io::{self, Write};
use constella_db::modules::protocol::{
  BincodeSerializer, ResponseData, Message, MessageType, ProtocolHandler
};
use constella_db::modules::db::Entity;
use tokio::net::TcpStream;
use uuid::Uuid;

pub fn format_rows(rows: Vec<Entity>) -> String {
  if rows.is_empty() {
    return "OK: 0 row(s)".to_string();
  }
  let mut out = format!("OK: {} row(s)\n", rows.len());
  for row in &rows {
    let fields: Vec<String> = row
      .data
      .iter()
      .map(|d| format!("{}={:?}", d.name, d.value))
      .collect();
    out.push_str(&format!("  {}\n", fields.join(", ")));
  }
  out.trim_end().to_string()
}

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
    print!("constella_db> ");
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
      Ok(resp) => {
        match resp.msg_type {
          MessageType::Response {
            sucess,
            message,
            data,
          } => {
            println!("Success: {}", sucess);

            if let Some(msg) = message {
              println!("{}", msg);
            }

            if let Some(data) = data {
              match data {
                ResponseData::Rows(rows) => {
                  let res = format_rows(rows);
                  println!("{}", res);
                }
                ResponseData::Tables(tables) => {
                  println!("Tables:");
                  for table in tables {
                    println!("  {}", table);
                  }
                }
              }
            }
          }
          _ => {
            eprintln!("Expected Response");
          }
        }
      }

      Err(e) => {
          eprintln!("recv error: {e}");
          break;
      }
    }
  }
}
