// TODOs:
// [] Make leader do work too
// [] Proper distribution algorithm
// [] Backup

pub mod node;
pub mod leader;
pub mod listener;
pub mod instruction;

use tokio::net::{TcpStream, TcpListener};
use clap::Parser;
use std::sync::Arc;
use tokio::sync::Mutex;

use protocol_module::{
  handler::{ReadHandler, WriteHandler},
  serializer::BincodeSerializer,
  message::{Message, MessageType},
};

use node::Node;
use listener::{get_local_ip, start_listener, job_message_handler, gateway_message_handler};

/// Commandline args for the node
#[derive(Parser, Debug)]
struct Args {
  #[arg(short, long)]
  port: Option<u32>,

  #[arg(short, long)]
  job_service: String,

  #[arg(short, long)]
  gateway: String,
}

/// Connects to the job scheduling service and runs the listener for
/// the job service responses
async fn connect_to_job_service(node: Arc<Mutex<Node>>, job_service_ip: &str) {
  let stream = TcpStream::connect(job_service_ip).await.unwrap();
  let (reader, writer) = stream.into_split();
  let read_handler = Arc::new(Mutex::new(
    ReadHandler::new(reader, Box::new(BincodeSerializer))
  ));
  let write_handler = Arc::new(Mutex::new(
    WriteHandler::new(writer, Box::new(BincodeSerializer))
  ));
  {
    let mut n = node.lock().await;
    n.job_service = Some((read_handler.clone(), write_handler.clone()));
  }

  // Spawn the listener for the job service
  let x = node.clone();
  tokio::spawn(async move {
    job_message_handler(read_handler, write_handler, x).await;
  });
}

async fn connect_to_gateway(node: Arc<Mutex<Node>>, gateway_ip: &str) {
  let stream = TcpStream::connect(gateway_ip).await.unwrap();
  let (reader, writer) = stream.into_split();
  let read_handler = Arc::new(Mutex::new(
    ReadHandler::new(reader, Box::new(BincodeSerializer))
  ));
  let write_handler = Arc::new(Mutex::new(
    WriteHandler::new(writer, Box::new(BincodeSerializer))
  ));

  // Send register msg
  {
    let n = node.lock().await;
    let mut w = write_handler.lock().await;
    let msg = Message::new(
      "test".to_string(),
      MessageType::Register,
      n.id.clone()
    );
    w.send(&msg).await.unwrap();
  }

  {
    let mut n = node.lock().await;
    n.gateway = Some((read_handler.clone(), write_handler.clone()));
  }

  // Spawn the listener for the gateway
  let x = node.clone();
  tokio::spawn(async move {
    gateway_message_handler(read_handler, write_handler, x).await;
  });
}

#[tokio::main]
async fn main() {
  let node = Arc::new(Mutex::new(Node::new()));

  let args = Args::parse();

  // Extract port from args
  let mut port = 0;
  if let Some(p) = args.port {
    port = p;
  }

  // Creating the main listener
  let ip = get_local_ip().unwrap();
  let listener = TcpListener::bind(
    format!("{}:{}", ip, port)
  ).await.unwrap();

  let bound_port = listener.local_addr().unwrap().port();
  let full_ip = format!("{}:{}", ip, bound_port);
  {
    let mut n = node.lock().await;
    n.id = full_ip.clone();
  }
  println!("[LOG] Node listening on {}", &full_ip);

  // Connecting to the job service
  let job_service_ip = args.job_service;
  connect_to_job_service(node.clone(), &job_service_ip).await;

  // Connecting to the gateway
  let gateway_ip = args.gateway;
  connect_to_gateway(node.clone(), &gateway_ip).await;

  start_listener(node, listener).await;
}
