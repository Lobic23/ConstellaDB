/// Here lies the functionality of the leader node

use tokio::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex;

use protocol_module::{
  handler::{ReadHandler, WriteHandler},
  message::{Message, MessageType},
  serializer::BincodeSerializer,
};

use crate::node::Node;
use crate::instruction::NodeStatus;

/// Distribute the message to the followers
/// TODO(slok): For now this distributes the message to every follower.
/// Later we need to develop on how the message should be distributed between
/// followers so that the efficiency is maximized
pub async fn distribute_message(msg: &Message, node: Arc<Mutex<Node>>) {
  let instructions = {
    let n = node.lock().await;
    n.instructions.clone()
  };

  let new_msg = Message::new(
    msg.id.clone(),
    MessageType::ExecCmd,
    msg.node_id.clone()
  )
    .with_command(
      msg
        .command
        .clone()
        .expect("Command should be present to distribute msg")
    );

  if let Some(instruction) = instructions.get(&msg.id) {
    for (ip, (_, write_handler)) in &instruction.followers {
      let mut handler = write_handler.lock().await;
      handler.send(&new_msg).await.unwrap();

      println!("[LOG] Sent to {}:\n{:#?}", &ip, &new_msg);
    }
  }

  if let Some(instructions) = instructions.get(&msg.id) {
    let s = instructions.nodes_status.lock().await;
    println!("[LOG] Instruction {} distributed to: {:?}", msg.id, s);
  }
}
