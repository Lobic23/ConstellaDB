use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use protocol_module::handler::ProtocolHandler;

pub struct Node {
  pub leader: bool,
  pub followers: HashMap<String, Arc<Mutex<ProtocolHandler>>>,
}

impl Node {
  pub fn new() -> Self {
    Self {
      leader: false,
      followers: HashMap::new(),
    }
  }
}
