use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use protocol_module::handler::{ReadHandler, WriteHandler};

use crate::instruction::Instruction;


/// A mega struct of states for node
/// Every thing that a node needs will be stored here
pub struct Node {
  pub leader: bool,
  pub id: String,
  pub job_service: Option<(Arc<Mutex<ReadHandler>>, Arc<Mutex<WriteHandler>>)>,
  pub job_table: HashMap<String, String>,
  pub instruction_owners: HashMap<String, Arc<Mutex<WriteHandler>>>,

  // For leader
  pub followers: HashMap<
    String,
    (Arc<Mutex<ReadHandler>>, Arc<Mutex<WriteHandler>>)
  >,
  pub instructions: HashMap<String, Instruction>,
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
