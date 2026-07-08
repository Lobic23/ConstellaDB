use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::net::TcpStream;

use db_module::Entity;
use protocol_module::{
  handler::{ReadHandler, WriteHandler},
  message::{Message, MessageType},
  serializer::BincodeSerializer,
};

use crate::node::Node;
use crate::listener::follower_message_handler;


/// Status of the worker node which determines the work is done or not
#[derive(Debug)]
pub struct NodeStatus {
  pub id: String,
  pub status: bool,   // true: done, false: not done
}


#[derive(Clone)]
pub struct Instruction {
  pub id: String,
  pub nodes_status: Arc<Mutex<Vec<NodeStatus>>>,        // Stores array of status of worker nodes

  pub client_write_handler: Arc<Mutex<WriteHandler>>,   // Writer stream of the client who requested this instruction
  pub followers: HashMap<
    String,                                             // Stores the followers reader and writer stream for communication/distribution
    (Arc<Mutex<ReadHandler>>, Arc<Mutex<WriteHandler>>) // follower_ip -> <reader, writer>
  >,

  pub response_message: Option<String>,
  pub response_rows: Option<Vec<Entity>>,
}

impl Instruction {
  pub async fn new(
    id: String,
    client_write_handler: Arc<Mutex<WriteHandler>>,
    follower_ips: Vec<String>,
    node: Arc<Mutex<Node>>
  ) -> Self {
    let followers = Self::connect_to_followers(node, follower_ips).await;

    // Create the node status for the followers
    let mut nodes_status = Vec::new();
    for (ip, _) in &followers {
      nodes_status.push(NodeStatus { id: ip.to_string(), status: false });
    }

    Self {
      id: id,
      nodes_status: Arc::new(Mutex::new(nodes_status)),

      client_write_handler: client_write_handler,
      followers: followers,

      response_message: None,
      response_rows: None,
    }
  }

  /// Connects to the follower's server as a client and stores the stream
  async fn connect_to_followers(
    node: Arc<Mutex<Node>>,
    ips: Vec<String>
  ) -> HashMap<String, (Arc<Mutex<ReadHandler>>, Arc<Mutex<WriteHandler>>)> {
    let mut map = HashMap::new();
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
      map.insert(
        follower_ip,
        (read_handler.clone(), write_handler.clone())
      );

      // Spawn the listener for all the follower as well
      let x = node.clone();
      tokio::spawn(async move {
        follower_message_handler(read_handler, write_handler, x).await;
      });
    }

    map
  }
}


/// Allocates a new instruction in the leader's node
pub async fn create_new_instruction(
  msg: &Message,
  node: Arc<Mutex<Node>>,
  write_handler: Arc<Mutex<WriteHandler>>,
  follower_ips: Vec<String>
) {
  let x = node.clone();
  let mut n = node.lock().await;
  n.instructions.insert(msg.id.clone(), Instruction::new(msg.id.clone(), write_handler, follower_ips, x).await);
  println!("[LOG] New instruction created: {}", msg.id);
}

/// Makes the status to true for node who has completed the task
pub async fn sucess_instruction_response(
  inst_id: &str,
  node_id: &str,
  node: Arc<Mutex<Node>>
) {
  let n = node.lock().await;
  if let Some(instruction) = n.instructions.get(inst_id) {
    let mut ns = instruction.nodes_status.lock().await;
    for node in ns.iter_mut() {
      if node.id == node_id {
        node.status = true;
        println!("[LOG] Instruction {} completed by: {:?}", inst_id, node);
      }
    }
  }
}

/// Checks if all the nodes has completed the task for the given instruction
pub async fn is_instruction_finished(
  inst_id: &str,
  node: Arc<Mutex<Node>>
) -> bool {
  let n = node.lock().await;
  if let Some(instruction) = n.instructions.get(inst_id) {
    let mut ns = instruction.nodes_status.lock().await;
    for node in ns.iter_mut() {
      if !node.status {
        return false;
      }
    }
    return true;
  }
  false
}
