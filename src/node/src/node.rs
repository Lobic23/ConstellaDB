use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use protocol_module::handler::{ReadHandler, WriteHandler};

pub struct NodeStatus {
  pub id: String,
  pub status: bool,   // true: done, false: not done
}

pub struct Node {
  pub leader: bool,
  pub id: String,
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
      followers: HashMap::new(),
      instructions: HashMap::new(),
    }
  }
}
