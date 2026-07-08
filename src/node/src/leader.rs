/// Here lies the functionality of the leader node

use std::sync::Arc;
use tokio::sync::Mutex;
use rand::seq::IteratorRandom;

use protocol_module::{
  message::{Message, MessageType},
};
use cmd_module::Command;

use crate::node::Node;

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
    match new_msg.command.as_ref().unwrap() {

      // Distribute to a random follower when insert is sent
      Command::Insert(_) => {
        let chosen = {
          let mut rng = rand::rng();

          instruction
           .followers
            .iter()
            .choose(&mut rng)
            .map(|(ip, (_, writer))| {
              (ip.clone(), writer.clone())
            })
        };

        if let Some((ip, write_handler)) = chosen {
          // Remove non selected followers from instruction's status
          {
            let mut status = instruction.nodes_status.lock().await;
            status.retain(|node| node.id == ip);
          }

          let mut handler = write_handler.lock().await;
          handler.send(&new_msg).await.unwrap();

          println!("[LOG] Sent INSERT to {}:\n{:#?}", ip, new_msg);
        }
      }

      _ => {
        for (ip, (_, write_handler)) in &instruction.followers {
          let mut handler = write_handler.lock().await;
          handler.send(&new_msg).await.unwrap();
          println!("[LOG] Sent to {}:\n{:#?}", ip, new_msg);
        }
      }
    }
  }
}
