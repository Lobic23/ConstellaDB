/// Here lies the functionality of the leader node

use std::sync::Arc;
use constella_db::modules::{cmd::Command, protocol::{Message, MessageType}};
use tokio::sync::Mutex;

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
      // TODO(slok): Distribute chunked data if multiple insert data is given
      Command::Insert(entities) => {
        let followers: Vec<_> = instruction
          .followers
          .iter()
          .map(|(ip, (_, writer))| (ip.clone(), writer.clone()))
          .collect();

        if followers.is_empty() {
          return;
        }

        let total = entities.len();
        let num_followers = followers.len();

        // ceil(total / num_followers)
        let chunk_size = total.div_ceil(num_followers);

        let mut used_followers = Vec::new();

        for ((ip, writer), chunk) in followers
          .into_iter()
          .zip(entities.chunks(chunk_size))
        {
          used_followers.push(ip.clone());

          let mut msg = new_msg.clone();
          *msg.command.as_mut().unwrap() = Command::Insert(chunk.to_vec());

          let mut handler = writer.lock().await;
          handler.send(&msg).await.unwrap();

          println!(
            "[LOG] Sent {} rows to {}",
            chunk.len(),
            ip
          );
        }

        {
          let mut status = instruction.nodes_status.lock().await;
          status.retain(|node| used_followers.contains(&node.id));
        }
      },

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
