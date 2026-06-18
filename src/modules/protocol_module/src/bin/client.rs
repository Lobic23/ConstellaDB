use protocol_module::{
    Message,
    MessageType,
    BincodeSerializer,
    ProtocolHandler,
};

use tokio::net::TcpStream;

fn debug(label: &str, bytes: &[u8]) {
    println!("\n=== {} ===", label);
    for b in bytes {
        print!("{:02x} ", b);
    }
    println!("\nlen = {}", bytes.len());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let stream = TcpStream::connect("127.0.0.1:8080").await?;

    let mut handler =
        ProtocolHandler::new(stream, Box::new(BincodeSerializer));

    // ---------------- MESSAGE ----------------
    let msg = Message::new(
        1,
        MessageType::Query,
        "client".to_string(),
    )
    .with_payload(b"Hello".to_vec());

    println!("\n[CLIENT SENDING]");
    println!("TEXT: {}", String::from_utf8_lossy(&msg.payload));

    debug("payload", &msg.payload);

    // ---------------- SEND ----------------
    handler.send(&msg).await?;

    println!("\nMessage sent!");

    // ---------------- RECEIVE ----------------
    let response = handler.receive().await?;

    println!("\n[CLIENT RECEIVED]");
    println!("TEXT: {}", String::from_utf8_lossy(&response.payload));

    debug("response payload", &response.payload);

    Ok(())
}