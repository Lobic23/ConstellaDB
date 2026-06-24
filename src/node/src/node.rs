use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use protocol_module::handler::{ReadHandler, WriteHandler};
use db_module::Entity;

// TODO(slok): Convert instruction id from u64 to String (uuid)

#[derive(Debug)]
pub struct NodeStatus {
  pub id: String,
  pub status: bool,   // true: done, false: not done
}

#[derive(Clone)]
pub struct Instruction {
  pub id: u64,
  pub nodes_status: Arc<Mutex<Vec<NodeStatus>>>,
  pub client_write_handler: Arc<Mutex<WriteHandler>>,

  pub response_message: Option<String>,
  pub response_rows: Option<Vec<Entity>>,
}

impl Instruction {
  pub fn new(id: u64, client_write_handler: Arc<Mutex<WriteHandler>>) -> Self {
    Self {
      id: id,
      nodes_status: Arc::new(Mutex::new(Vec::new())),
      client_write_handler: client_write_handler,

      response_message: None,
      response_rows: None,
    }
  }
}

pub struct Node {
  pub leader: bool,
  pub id: String,
  pub job_service: Option<(Arc<Mutex<ReadHandler>>, Arc<Mutex<WriteHandler>>)>,
  pub job_table: HashMap<String, u64>,
  pub instruction_owners: HashMap<u64, Arc<Mutex<WriteHandler>>>,

  // For leader
  pub followers: HashMap<
    String,
    (Arc<Mutex<ReadHandler>>, Arc<Mutex<WriteHandler>>)
  >,
  pub instructions: HashMap<u64, Instruction>,
}

impl Node {
  pub fn new() -> Self {
    Self {
      leader: false,
      id: "".to_string(),
      job_service: None,
      job_table: HashMap::new(),
      instruction_owners: HashMap::new(),
      followers: HashMap::new(),
      instructions: HashMap::new(),
    }
  }
}
