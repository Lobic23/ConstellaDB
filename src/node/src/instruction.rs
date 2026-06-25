use std::sync::Arc;
use tokio::sync::Mutex;

use protocol_module::handler::WriteHandler;
use protocol_module::message::Message;
use db_module::Entity;

use crate::node::Node;


/// Status of the worker node which determines the work is done or not
#[derive(Debug)]
pub struct NodeStatus {
  pub id: String,
  pub status: bool,   // true: done, false: not done
}


#[derive(Clone)]
pub struct Instruction {
  pub id: String,
  pub nodes_status: Arc<Mutex<Vec<NodeStatus>>>,       // Stores array of status of worker nodes
  pub client_write_handler: Arc<Mutex<WriteHandler>>,  // Writer stream of the client who requested this instruction

  pub response_message: Option<String>,
  pub response_rows: Option<Vec<Entity>>,
}

impl Instruction {
  pub fn new(id: String, client_write_handler: Arc<Mutex<WriteHandler>>) -> Self {
    Self {
      id: id,
      nodes_status: Arc::new(Mutex::new(Vec::new())),
      client_write_handler: client_write_handler,

      response_message: None,
      response_rows: None,
    }
  }
}


/// Allocates a new instruction in the leader's node
pub async fn create_new_instruction(
  msg: &Message,
  node: Arc<Mutex<Node>>,
  write_handler: Arc<Mutex<WriteHandler>>
) {
  let mut n = node.lock().await;
  n.instructions.insert(msg.id.clone(), Instruction::new(msg.id.clone(), write_handler));
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
