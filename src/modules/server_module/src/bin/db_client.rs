use server_module::codec::{read_message, write_message};
use protocol_module::message::{Message, MessageType};
use tokio::net::TcpStream;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7878".to_string());

    let node_id = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "client".to_string());

    let stream = TcpStream::connect(&addr).await.unwrap();
    println!("[db_client] connected to {} as '{}'", addr, node_id);
    println!("[db_client] type SQL and press Enter. Ctrl+C to quit.\n");

    let (mut reader, mut writer) = stream.into_split();
    let mut next_id: u64 = 0;
    let stdin = io::stdin();

    loop {
        print!("constelladb> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => { eprintln!("stdin error: {e}"); break; }
        }

        let sql = input.trim();
        if sql.is_empty() { continue; }

        let msg = Message::new(next_id, MessageType::Query, node_id.clone())
            .with_payload(sql.as_bytes().to_vec());
        next_id += 1;

        if let Err(e) = write_message(&mut writer, &msg).await {
            eprintln!("send error: {e}");
            break;
        }

        match read_message(&mut reader).await {
            Ok(resp) => {
                let text = String::from_utf8_lossy(&resp.payload);
                println!("{}", text);
            }
            Err(e) => {
                eprintln!("recv error: {e}");
                break;
            }
        }
    }
}