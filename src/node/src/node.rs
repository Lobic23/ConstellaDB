use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use protocol_module::handler::{ReadHandler, WriteHandler};

// TODO(slok): Convert instruction id from u64 to String (uuid)

#[derive(Debug)]
pub struct NodeStatus {
  pub id: String,
  pub status: bool,   // true: done, false: not done
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
  pub instructions: HashMap<
    u64,
    (Arc<Mutex<Vec<NodeStatus>>>, Arc<Mutex<WriteHandler>>)
  >,
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
