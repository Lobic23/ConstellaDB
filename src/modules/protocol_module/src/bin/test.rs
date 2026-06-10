use protocol_module::{
    Message, MessageType, Command, BincodeSerializer, Serializer, ProtocolHandler
};

fn hex_format(bytes: &[u8]) -> String {
    bytes.iter()
        .take(8)
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let command = Command::Select("Select * FROM users".to_string());
    println!("Command: {:?}", command);

    let message = Message::new(1, MessageType::Query, "node1".to_string())
        .with_command(command)
        .with_payload(b"extra data".to_vec());
    println!("Message Created: ");
    println!(" ID: {}", message.id);
    println!(" Type: {:?}", message.msg_type);
    println!(" Node: {}", message.node_id);
    println!(" Command: {:?}", message.command);

    let serializer = BincodeSerializer;
    let bytes = serializer.serialize(&message).unwrap();

    println!("Serialized:");
    println!(" Size: {} bytes", bytes.len());
    println!(" Bytes (hex): {}", hex_format(&bytes));

    let received = serializer.deserialize(&bytes).unwrap();

    println!("Deserialized: ");
    println!("  - ID: {}", received.id);
    println!("  - Type: {:?}", received.msg_type);
    println!("  - Node: {}", received.node_id);
    println!("  - Command: {:?}\n", received.command);
}
