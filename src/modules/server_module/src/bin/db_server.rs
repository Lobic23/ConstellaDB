use server_module::codec::{read_message, write_message};
use server_module::db::execute;
use cmd_module::parse_command;
use db_module::Engine;
use protocol_module::message::{Message, MessageType};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:7878";

    // Engine is shared across all client tasks
    let engine = Arc::new(Mutex::new(Engine::new()));

    let listener = TcpListener::bind(addr).await.unwrap();
    println!("[db_server] listening on {}", addr);

    loop {
        let (stream, peer) = listener.accept().await.unwrap();
        println!("[db_server] client connected: {}", peer);

        let engine = Arc::clone(&engine);

        tokio::spawn(async move {
            let (mut reader, mut writer) = stream.into_split();

            loop {
                match read_message(&mut reader).await {
                    Ok(msg) => {
                        // Only handle Query messages
                        if msg.msg_type != MessageType::Query {
                            eprintln!("[db_server] unexpected msg type from {}", peer);
                            continue;
                        }

                        let sql = match String::from_utf8(msg.payload.clone()) {
                            Ok(s)  => s,
                            Err(_) => {
                                let _ = send_error(&mut writer, msg.id, "invalid UTF-8 payload").await;
                                continue;
                            }
                        };

                        println!("[db_server] {} >>> {}", peer, sql.trim());

                        let result = match parse_command(&sql) {
                            Ok(cmd) => {
                                let mut eng = engine.lock().await;
                                execute(&mut eng, cmd)
                            }
                            Err(e) => format!("Parse error: {e}"),
                        };

                        println!("[db_server] {}", result);

                        let response = Message::new(msg.id, MessageType::Response, "server".to_string())
                            .with_payload(result.into_bytes());

                        if let Err(e) = write_message(&mut writer, &response).await {
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

async fn send_error(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    id: u64,
    msg: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let err_msg = Message::new(id, MessageType::Error, "server".to_string())
        .with_payload(msg.as_bytes().to_vec());
    write_message(writer, &err_msg).await
}