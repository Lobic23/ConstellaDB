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
  pub job_service: Option<(Arc<Mutex<ReadHandler>>, Arc<Mutex<WriteHandler>>)>, // Holds the reader and writer stream of job service for communication
  pub gateway: Option<(Arc<Mutex<ReadHandler>>, Arc<Mutex<WriteHandler>>)>,     // Holds the reader and writer stream of gateway for communication
  pub job_table: HashMap<String, String>,                                       // job_id -> instruction_id
  pub instruction_owners: HashMap<String, Arc<Mutex<WriteHandler>>>,            // Stores the writer stream of node who gave the instruction to this node i.e leader
                                                                                // instruction_id -> another node writer stream
  // For leader
  pub instructions: HashMap<String, Instruction>,                               // Stores all the instructions handled by the leader
}

impl Node {
  pub fn new() -> Self {
    Self {
      leader: false,
      id: "".to_string(),
      job_service: None,
      gateway: None,
      job_table: HashMap::new(),
      instruction_owners: HashMap::new(),
      instructions: HashMap::new(),
    }
  }
}
