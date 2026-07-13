use protocol_module::{
    Message,
    MessageType,
    BincodeSerializer,
    ProtocolHandler,
};

use tokio::net::TcpListener;

fn debug(label: &str, bytes: &[u8]) {
    println!("\n=== {} (HEX) ===", label);
    for b in bytes {
        print!("{:02x} ", b);
    }
    println!("\nlen = {}", bytes.len());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server listening...");

    let (stream, _) = listener.accept().await?;
    let mut handler =
        ProtocolHandler::new(stream, Box::new(BincodeSerializer));

    // ---------------- RECEIVE ----------------
    let msg = handler.receive().await?;

    println!("\n========= SERVER RECEIVED =========");

    let plain_text = String::from_utf8_lossy(&msg.payload);

    println!("PLAIN TEXT: {}", plain_text);

    debug("PAYLOAD BYTES", &msg.payload);

    // ---------------- RESPONSE ----------------
    let reply = Message::new(
        2.to_string(),
        MessageType::Response,
        "server".to_string(),
    )
    .with_payload(b"Hello back!".to_vec());

    let reply_text = String::from_utf8_lossy(&reply.payload);
    println!("\nREPLY PLAIN TEXT: {}", reply_text);

    debug("REPLY PAYLOAD BYTES", &reply.payload);

    handler.send(&reply).await?;

    println!("\nReply sent");

    Ok(())
}