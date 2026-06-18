use crate::message::Message;

pub trait Serializer: Send + Sync {
  fn serialize(&self, message: &Message) -> Result<Vec<u8>, String>;
  fn deserialize(&self, data: &[u8]) -> Result<Message, String>;
}

pub struct BincodeSerializer;

impl Serializer for BincodeSerializer {
  fn serialize(&self, message: &Message) -> Result<Vec<u8>, String> {
    bincode::serialize(message).map_err(|e| format!("Serialization error: {}", e))
  }

  fn deserialize(&self, data: &[u8]) -> Result<Message, String> {
    bincode::deserialize(data).map_err(|e| format!("Deserialization error: {}", e))
  }
}
