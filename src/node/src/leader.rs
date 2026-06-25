/// Here lies the functionality of the leader node

use tokio::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex;

use protocol_module::{
  handler::{ReadHandler, WriteHandler},
  message::Message,
  serializer::BincodeSerializer,
};

use crate::node::Node;
use crate::instruction::NodeStatus;
use crate::listener::connection_listener;


/// Connects to the follower's server as a client and stores the stream
pub async fn connect_to_followers(node: Arc<Mutex<Node>>, ips: Vec<String>) {
  for follower_ip in ips {
    let stream = TcpStream::connect(&follower_ip).await.unwrap();
    let (reader, writer) = stream.into_split();

    let read_handler = Arc::new(Mutex::new(
      ReadHandler::new(reader, Box::new(BincodeSerializer))
    ));
    let write_handler = Arc::new(Mutex::new(
      WriteHandler::new(writer, Box::new(BincodeSerializer))
    ));

    println!("[LOG] Connected to follower {}", &follower_ip);

    // Save the follower's streams
    {
      let mut n = node.lock().await;
      n.followers.insert(
        follower_ip,
        (read_handler.clone(), write_handler.clone())
      );
    }

    // Spawn the listener for all the follower as well
    let x = node.clone();
    tokio::spawn(async move {
      connection_listener(read_handler, write_handler, x).await;
    });
  }
}


/// Distribute the message to the followers
/// TODO(slok): For now this distributes the message to every follower.
/// Later we need to develop on how the message should be distributed between
/// followers so that the efficiency is maximized
pub async fn distribute_message(msg: &Message, node: Arc<Mutex<Node>>) {
  let followers = {
    let n = node.lock().await;
    n.followers.clone()
  };
  let instructions = {
    let n = node.lock().await;
    n.instructions.clone()
  };

  for (ip, (_, write_handler)) in followers {
    let mut handler = write_handler.lock().await;
    handler.send(msg).await.unwrap();

    // Save the node to the instruction
    if let Some(instruction) = instructions.get(&msg.id) {
      let mut s = instruction.nodes_status.lock().await;
      s.push(NodeStatus { id: ip, status: false });
    }
  }

  if let Some(instructions) = instructions.get(&msg.id) {
    let s = instructions.nodes_status.lock().await;
    println!("[LOG] Instruction {} distributed to: {:?}", msg.id, s);
  }
}

