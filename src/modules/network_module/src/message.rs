use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    Ping, 
    Pong,
    Put { key: String, value: String },
    Get { key: String },
    Ok,
    Err { reason: String },
}