use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use cmd_module::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
  Query,                           // Request to execute database query
  Lead { followers: Vec<String> }, // Sent to the leader to lead the cmd
  ExecCmd,                         // Command to execute (parsed query)
  Response,                        // Reply to query
  Register,                        // Register the node
  Error,                           // Error notification
  JobInit     { job_id: String },  // New Job Initialized
  JobComplete { job_id: String },  // Job is completed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
  pub id: String,
  pub msg_type: MessageType,
  pub command: Option<Command>,
  pub payload: Vec<u8>,
  pub timestamp: u64,
  pub node_id: String,
}

impl Message {
  pub fn new(id: String, msg_type: MessageType, node_id: String) -> Self {
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_secs();

    Message {
      id,
      msg_type,
      command: None,
      payload: Vec::new(),
      timestamp,
      node_id,
    }
  }

  pub fn with_command(mut self, command: Command) -> Self {
    self.command = Some(command);
    self
  }

  pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
    self.payload = payload;
    self
  }

  pub fn is_query(&self) -> bool {
    self.msg_type == MessageType::Query
  }

  pub fn is_response(&self) -> bool {
    self.msg_type == MessageType::Response
  }
}
